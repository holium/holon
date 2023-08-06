use std::collections::VecDeque;
use std::{collections::HashMap, sync::Arc};

use crate::net::connections::build_connection;
use crate::types::*;

use aes_gcm_siv::Nonce;
use elliptic_curve::ecdh::EphemeralSecret;
use elliptic_curve::PublicKey;
use ethers::prelude::k256::{self, Secp256k1};
use futures::StreamExt;
use ring::signature::{self, Ed25519KeyPair};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::task::JoinSet;
use tokio_tungstenite::{accept_async, connect_async, MaybeTlsStream, WebSocketStream};

mod connections;

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

type Peers = Arc<RwLock<HashMap<String, Peer>>>;
type WebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;
type ErrorShuttle = oneshot::Sender<Result<(), NetworkError>>;

pub struct Peer {
    pub networking_address: String,
    pub is_ward: bool,
    pub sender: mpsc::UnboundedSender<(NetworkMessage, ErrorShuttle)>,
    pub handler: mpsc::UnboundedSender<Vec<u8>>,
    pub error: Option<NetworkError>,
}

/// parsed from Binary websocket message on an Indirect route
#[derive(Debug, Serialize, Deserialize)]
pub enum NetworkMessage {
    Ack(u64),
    Nack(u64),
    Msg {
        from: String,
        to: String,
        contents: Vec<u8>,
    },
    Raw(WrappedMessage),
    Handshake(Handshake),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkError {
    Timeout,
    Offline,
}

/// contains identity and encryption keys, used in initial handshake.
/// parsed from Text websocket message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Handshake {
    from: String,
    target: String,
    id_signature: Vec<u8>,
    ephemeral_public_key: Vec<u8>,
    ephemeral_public_key_signature: Vec<u8>,
    nonce: Vec<u8>,
}

pub async fn networking(
    our: Identity,
    our_ip: String,
    keypair: Ed25519KeyPair,
    pki: OnchainPKI,
    kernel_message_tx: MessageSender,
    print_tx: PrintSender,
    mut message_rx: MessageReceiver,
) {
    let peers: Peers = Arc::new(RwLock::new(HashMap::new()));
    let keypair = Arc::new(keypair);

    let listener_handle = match &our.ws_routing {
        None => {
            // connect to router(s)
            tokio::spawn(connect_to_routers(
                our.clone(),
                keypair.clone(),
                our_ip.clone(),
                pki.clone(),
                peers.clone(),
                kernel_message_tx.clone(),
            ))
        }
        Some((_ip, port)) => {
            // spawn the listener
            tokio::spawn(receive_incoming_connections(
                our.clone(),
                keypair.clone(),
                *port,
                pki.clone(),
                peers.clone(),
                kernel_message_tx.clone(),
            ))
        }
    };

    tokio::select! {
        _listener = listener_handle => (),
        _sender = async {
            while let Some(wm) = message_rx.recv().await {
                if wm.message.wire.target_ship == our.name {
                    handle_incoming_message(&our, wm.message, peers.clone(), print_tx.clone()).await;
                    continue;
                }
                let start = std::time::Instant::now();
                // TODO can parallelize this
                let result = message_to_peer(
                    our.clone(),
                    our_ip.clone(),
                    keypair.clone(),
                    pki.clone(),
                    peers.clone(),
                    wm.clone(),
                    kernel_message_tx.clone(),
                ).await;

                let end = std::time::Instant::now();
                let elapsed = end.duration_since(start);
                let _ = print_tx.send(Printout {
                    verbosity: 1,
                    content: format!("message_to_peer took {:?}", elapsed),
                }).await;

                match result {
                    Ok(()) => continue,
                    Err(e) => {
                        let _ = kernel_message_tx
                            .send(make_kernel_response(&our, wm, e))
                            .await;
                    }
                }
            }
        } => (),
    }
}

/// only used if indirect. should live forever unless we can't connect to any routers
async fn connect_to_routers(
    our: Identity,
    keypair: Arc<Ed25519KeyPair>,
    our_ip: String,
    pki: OnchainPKI,
    peers: Peers,
    kernel_message_tx: MessageSender,
) {
    // first accumulate as many connections as possible
    let mut routers = JoinSet::<Result<String, tokio::task::JoinError>>::new();
    for router_name in &our.allowed_routers {
        println!("trying for router {router_name}\r\n");
        if let Some(router_id) = pki.read().await.get(router_name) {
            if let Some((ip, port)) = &router_id.ws_routing {
                if let Ok(ws_url) = make_ws_url(&our_ip, ip, port) {
                    if let Ok((websocket, _response)) = connect_async(ws_url).await {
                        // this is a real and functional router! woohoo
                        if let Ok(active_peer) = build_connection(
                            our.clone(),
                            keypair.clone(),
                            Some(router_id.clone()),
                            None,
                            pki.clone(),
                            peers.clone(),
                            websocket,
                            kernel_message_tx.clone(),
                        )
                        .await
                        {
                            println!("connected to router {router_name}!\r\n");
                            routers.spawn(active_peer);
                        }
                    }
                }
            }
        }
    }
    // then, poll those connections in parallel
    // if any of them fail, we will try to reconnect
    // TODO learn more about joinerrors and if we can't just unwrap
    while let Some(err) = routers.join_next().await {
        let router_name = err.unwrap().unwrap();
        println!("lost router {}!\r\n", router_name);
        // try to reconnect
        if let Some(router_id) = pki.read().await.get(&router_name) {
            if let Some((ip, port)) = &router_id.ws_routing {
                if let Ok(ws_url) = make_ws_url(&our_ip, ip, port) {
                    if let Ok((websocket, _response)) = connect_async(ws_url).await {
                        // this is a real and functional router! woohoo
                        if let Ok(active_peer) = build_connection(
                            our.clone(),
                            keypair.clone(),
                            Some(router_id.clone()),
                            None,
                            pki.clone(),
                            peers.clone(),
                            websocket,
                            kernel_message_tx.clone(),
                        )
                        .await
                        {
                            println!("connected to router {router_name}!\r\n");
                            routers.spawn(active_peer);
                        }
                    }
                }
            }
        }
    }
    // if no connections exist, fatal end!
}

/// only used if direct. should live forever
async fn receive_incoming_connections(
    our: Identity,
    keypair: Arc<Ed25519KeyPair>,
    port: u16,
    pki: OnchainPKI,
    peers: Peers,
    kernel_message_tx: MessageSender,
) {
    println!("receive_incoming_connections\r\n");
    let tcp = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect(format!("fatal error: can't listen on port {port}").as_str());

    while let Ok((stream, _socket_addr)) = tcp.accept().await {
        match accept_async(MaybeTlsStream::Plain(stream)).await {
            Ok(websocket) => {
                tokio::spawn(connections::build_connection(
                    our.clone(),
                    keypair.clone(),
                    None,
                    None,
                    pki.clone(),
                    peers.clone(),
                    websocket,
                    kernel_message_tx.clone(),
                ));
            }
            // ignore connections we failed to accept
            Err(_) => {}
        }
    }
}

async fn message_to_peer(
    our: Identity,
    our_ip: String,
    keypair: Arc<Ed25519KeyPair>,
    pki: OnchainPKI,
    peers: Peers,
    wm: WrappedMessage,
    kernel_message_tx: MessageSender,
) -> Result<(), NetworkError> {
    println!("message_to_peer\r\n");
    let target = &wm.message.wire.target_ship;
    let mut peers_write = peers.write().await;
    match peers_write.get_mut(target) {
        Some(peer) => {
            // if we have the peer, simply send the message to their sender
            let (result_tx, result_rx) = oneshot::channel::<Result<(), NetworkError>>();
            let _ = peer
                .sender
                .send((NetworkMessage::Raw(wm.clone()), result_tx));
            drop(peers_write);
            // if, after this, they are still in our peermap, message was sent OR timeout.
            // otherwise, they're offline
            result_rx.await.unwrap_or(Err(NetworkError::Timeout))
        }
        None => {
            drop(peers_write);
            // search PKI for peer and attempt to create a connection, then resend
            match pki.read().await.get(target) {
                // peer does not exist in PKI!
                None => {
                    return Err(NetworkError::Offline);
                }
                // peer exists in PKI
                Some(peer_id) => {
                    match &peer_id.ws_routing {
                        //
                        //  can connect directly to peer
                        //
                        Some((ip, port)) => {
                            if let Ok(ws_url) = make_ws_url(&our_ip, ip, port) {
                                if let Ok((websocket, _response)) = connect_async(ws_url).await {
                                    let (result_tx, result_rx) =
                                        oneshot::channel::<Result<(), NetworkError>>();
                                    let _ = connections::build_connection(
                                        our.clone(),
                                        keypair,
                                        Some(peer_id.clone()),
                                        Some((NetworkMessage::Raw(wm.clone()), result_tx)),
                                        pki.clone(),
                                        peers.clone(),
                                        websocket,
                                        kernel_message_tx.clone(),
                                    )
                                    .await;
                                    return result_rx.await.unwrap_or(Err(NetworkError::Timeout));
                                }
                            }
                            return Err(NetworkError::Offline);
                        }
                        //
                        //  peer does not have direct routing info, need to use router
                        //
                        None => {
                            let mut routers_to_try =
                                VecDeque::from(peer_id.allowed_routers.clone());
                            while let Some(router) = routers_to_try.pop_front() {
                                let (result_tx, result_rx) =
                                    oneshot::channel::<Result<(), NetworkError>>();
                                let res = connections::build_routed_connection(
                                    our.clone(),
                                    our_ip.clone(),
                                    keypair.clone(),
                                    router.clone(),
                                    (NetworkMessage::Raw(wm.clone()), result_tx),
                                    pki.clone(),
                                    peers.clone(),
                                    kernel_message_tx.clone(),
                                )
                                .await;
                                if result_rx
                                    .await
                                    .unwrap_or(Err(NetworkError::Timeout))
                                    .is_ok()
                                {
                                    return Ok(());
                                }
                                if let Ok(Some(new_router)) = res {
                                    routers_to_try.push_back(new_router);
                                }
                            }
                            //
                            // we tried all available routers and none of them worked!
                            //
                            return Err(NetworkError::Offline);
                        }
                    }
                }
            }
        }
    }
}

async fn handle_incoming_message(
    our: &Identity,
    message: Message,
    peers: Peers,
    print_tx: PrintSender,
) {
    if message.wire.source_ship != our.name {
        let _ = print_tx
            .send(Printout {
                verbosity: 0,
                content: format!(
                    "\x1b[3;32m{}: {}\x1b[0m",
                    message.wire.source_ship,
                    message
                        .payload
                        .json
                        .as_ref()
                        .unwrap_or(&serde_json::Value::Null),
                ),
            })
            .await;
    } else {
        // available commands: peers
        match message
            .payload
            .json
            .as_ref()
            .unwrap_or(&serde_json::Value::Null)
        {
            serde_json::Value::String(s) => {
                if s == "peers" {
                    let peer_read = peers.read().await;
                    let _ = print_tx
                        .send(Printout {
                            verbosity: 0,
                            content: format!("{:?}", peer_read.keys()),
                        })
                        .await;
                }
            }
            _ => {
                let _ = print_tx
                    .send(Printout {
                        verbosity: 1,
                        content: "ws: got unknown command".into(),
                    })
                    .await;
            }
        }
    }
}

/*
 *  networking utils
 */

fn make_ws_url(our_ip: &str, ip: &str, port: &u16) -> Result<url::Url, NetworkingError> {
    // if we have the same public IP as target, route locally,
    // otherwise they will appear offline due to loopback stuff
    let ip = if our_ip == ip { "localhost" } else { ip };
    match url::Url::parse(&format!("ws://{}:{}/ws", ip, port)) {
        Ok(v) => Ok(v),
        Err(_) => Err(NetworkingError::PeerOffline),
    }
}

fn make_kernel_response(our: &Identity, wm: WrappedMessage, err: NetworkError) -> WrappedMessage {
    WrappedMessage {
        id: wm.id,
        rsvp: None,
        message: Message {
            message_type: MessageType::Response,
            wire: Wire {
                source_ship: our.name.clone(),
                source_app: "net".into(),
                target_ship: our.name.clone(),
                target_app: wm.message.wire.source_app,
            },
            payload: Payload {
                json: Some(serde_json::to_value(err).unwrap()),
                bytes: None,
            },
        },
    }
}

/*
 *  handshake utils
 */

/// read one message from websocket stream and parse it as a handshake.
async fn get_handshake(websocket: &mut WebSocket) -> Result<Handshake, String> {
    let handshake_text = websocket
        .next()
        .await
        .ok_or("handshake failed")?
        .map_err(|e| format!("{}", e))?
        .into_text()
        .map_err(|e| format!("{}", e))?;
    let handshake: Handshake =
        serde_json::from_str(&handshake_text).map_err(|_| "got bad handshake")?;
    Ok(handshake)
}

/// take in handshake and PKI identity, and confirm that the handshake is valid.
/// takes in optional nonce, which must be the one that connection initiator created.
fn validate_handshake(
    handshake: &Handshake,
    their_id: &Identity,
    nonce: Vec<u8>,
) -> Result<(Arc<PublicKey<Secp256k1>>, Arc<Nonce>), String> {
    let their_networking_key = signature::UnparsedPublicKey::new(
        &signature::ED25519,
        hex::decode(&their_id.networking_key).map_err(|_| "failed to decode networking key")?,
    );

    if !(their_networking_key
        .verify(
            &serde_json::to_vec(&their_id).map_err(|_| "failed to serialize their identity")?,
            &handshake.id_signature,
        )
        .is_ok()
        && their_networking_key
            .verify(
                &handshake.ephemeral_public_key,
                &handshake.ephemeral_public_key_signature,
            )
            .is_ok())
    {
        // improper signatures on identity info, close connection
        return Err("got improperly signed networking info".into());
    }

    let their_ephemeral_pk =
        match PublicKey::<Secp256k1>::from_sec1_bytes(&handshake.ephemeral_public_key) {
            Ok(v) => Arc::new(v),
            Err(_) => return Err("error".into()),
        };

    // assign nonce based on our role in the connection
    let nonce = Arc::new(*Nonce::from_slice(&nonce));
    return Ok((their_ephemeral_pk, nonce));
}

/// given an identity and networking key-pair, produces a handshake message along
/// with an ephemeral secret to be used in a specific connection.
fn make_secret_and_handshake(
    our: &Identity,
    keypair: Arc<Ed25519KeyPair>,
    target: String,
) -> (Arc<EphemeralSecret<Secp256k1>>, Handshake) {
    // produce ephemeral keys for DH exchange and subsequent symmetric encryption
    let ephemeral_secret = Arc::new(EphemeralSecret::<k256::Secp256k1>::random(
        &mut rand::rngs::OsRng,
    ));
    let ephemeral_public_key = ephemeral_secret.public_key();
    // sign the ephemeral public key with our networking management key
    let signed_pk = keypair
        .sign(&ephemeral_public_key.to_sec1_bytes())
        .as_ref()
        .to_vec();
    let signed_id = keypair
        .sign(&serde_json::to_vec(our).unwrap())
        .as_ref()
        .to_vec();

    let mut iv = [0u8; 12];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut iv);
    let nonce = iv.to_vec();

    let handshake = Handshake {
        from: our.name.clone(),
        target: target.clone(),
        id_signature: signed_id,
        ephemeral_public_key: ephemeral_public_key.to_sec1_bytes().to_vec(),
        ephemeral_public_key_signature: signed_pk,
        // if we are connection initiator, send nonce inside message
        nonce,
    };

    (ephemeral_secret, handshake)
}
