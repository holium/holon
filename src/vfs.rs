use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::types::*;

const VFS_PERSIST_STATE_CHANNEL_CAPACITY: usize = 5;
const VFS_TASK_DONE_CHANNEL_CAPACITY: usize = 5;
const VFS_RESPONSE_CHANNEL_CAPACITY: usize = 2;

type ResponseRouter = HashMap<u64, MessageSender>;
#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
enum Key {
    Dir { id: u64 },
    File { id: u128 },
    // ...
}
type KeyToEntry = HashMap<Key, Entry>;
type PathToKey = HashMap<String, Key>;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Vfs {
    key_to_entry: KeyToEntry,
    path_to_key: PathToKey,
}
type IdentifierToVfs = HashMap<String, Arc<Mutex<Vfs>>>;
type IdentifierToVfsSerializable = HashMap<String, Vfs>;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Entry {
    name: String,
    full_path: String, //  full_path, ending with `/` for dir
    entry_type: EntryType,
    // ...  //  general metadata?
}
#[derive(Clone, Debug, Deserialize, Serialize)]
enum EntryType {
    Dir { parent: Key, children: HashSet<Key> },
    File { parent: Key }, //  hash could be generalized to `location` if we want to be able to point at, e.g., remote files
                          // ...  //  symlinks?
}

impl Vfs {
    fn new() -> Self {
        let mut key_to_entry: KeyToEntry = HashMap::new();
        let mut path_to_key: PathToKey = HashMap::new();
        let root_path: String = "/".into();
        let root_key = Key::Dir { id: 0 };
        key_to_entry.insert(
            root_key.clone(),
            Entry {
                name: root_path.clone(),
                full_path: root_path.clone(),
                entry_type: EntryType::Dir {
                    parent: root_key.clone(),
                    children: HashSet::new(),
                },
            },
        );
        path_to_key.insert(root_path.clone(), root_key.clone());
        Vfs {
            key_to_entry,
            path_to_key,
        }
    }
}

fn make_dir_name(full_path: &str) -> (String, String) {
    if full_path == "/" {
        return ("/".into(), "".into()); //  root case
    }
    let mut split_path: Vec<&str> = full_path.split("/").collect();
    let _ = split_path.pop();
    let name = format!("{}/", split_path.pop().unwrap());
    let path = split_path.join("/");
    let path = if path == "" {
        "/".into()
    } else {
        format!("{}/", path)
    };
    (name, path)
}

fn make_file_name(full_path: &str) -> (String, String) {
    let mut split_path: Vec<&str> = full_path.split("/").collect();
    let name = split_path.pop().unwrap();
    let path = format!("{}/", split_path.join("/"));
    (name.into(), path)
}

fn make_error_message(
    our_node: String,
    id: u64,
    source: Address,
    error: VfsError,
) -> KernelMessage {
    KernelMessage {
        id,
        source: Address {
            node: our_node,
            process: VFS_PROCESS_ID.clone(),
        },
        target: source,
        rsvp: None,
        message: Message::Response((
            Response {
                ipc: Some(serde_json::to_string(&error).unwrap()), //  TODO: handle error?
                metadata: None,
            },
            None,
        )),
        payload: None,
        signed_capabilities: None,
    }
}

async fn state_to_bytes(state: &IdentifierToVfs) -> Vec<u8> {
    let mut serializable: IdentifierToVfsSerializable = HashMap::new();
    for (id, vfs) in state.iter() {
        let vfs = vfs.lock().await;
        serializable.insert(id.clone(), (*vfs).clone());
    }
    bincode::serialize(&serializable).unwrap()
}

fn bytes_to_state(bytes: &Vec<u8>, state: &mut IdentifierToVfs) {
    let serializable: IdentifierToVfsSerializable = bincode::deserialize(&bytes).unwrap();
    for (id, vfs) in serializable.into_iter() {
        state.insert(id, Arc::new(Mutex::new(vfs)));
    }
}

async fn persist_state(our_node: String, send_to_loop: &MessageSender, state: &IdentifierToVfs) {
    let _ = send_to_loop
        .send(KernelMessage {
            id: rand::random(),
            source: Address {
                node: our_node.clone(),
                process: VFS_PROCESS_ID.clone(),
            },
            target: Address {
                node: our_node,
                process: FILESYSTEM_PROCESS_ID.clone(),
            },
            rsvp: None,
            message: Message::Request(Request {
                inherit: true,
                expects_response: Some(5), // TODO evaluate
                ipc: Some(
                    serde_json::to_string(&FsAction::SetState(VFS_PROCESS_ID.clone())).unwrap(),
                ),
                metadata: None,
            }),
            payload: Some(Payload {
                mime: None,
                bytes: state_to_bytes(state).await,
            }),
            signed_capabilities: None,
        })
        .await;
}

async fn load_state_from_reboot(
    our_node: String,
    send_to_loop: &MessageSender,
    mut recv_from_loop: MessageReceiver,
    drive_to_vfs: &mut IdentifierToVfs,
    id: u64,
) -> bool {
    let _ = send_to_loop
        .send(KernelMessage {
            id,
            source: Address {
                node: our_node.clone(),
                process: VFS_PROCESS_ID.clone(),
            },
            target: Address {
                node: our_node.clone(),
                process: FILESYSTEM_PROCESS_ID.clone(),
            },
            rsvp: None,
            message: Message::Request(Request {
                inherit: true,
                expects_response: Some(5), // TODO evaluate
                ipc: Some(
                    serde_json::to_string(&FsAction::GetState(VFS_PROCESS_ID.clone())).unwrap(),
                ),
                metadata: None,
            }),
            payload: None,
            signed_capabilities: None,
        })
        .await;
    let km = recv_from_loop.recv().await;
    let Some(km) = km else {
        return false;
    };

    let KernelMessage {
        message, payload, ..
    } = km;
    let Message::Response((Response { ipc, .. }, None)) = message else {
        return false;
    };
    let Ok(Ok(FsResponse::GetState)) =
        serde_json::from_str::<Result<FsResponse, FsError>>(&ipc.unwrap_or_default())
    else {
        return false;
    };
    let Some(payload) = payload else {
        panic!("");
    };
    bytes_to_state(&payload.bytes, drive_to_vfs);

    return true;
}

pub async fn vfs(
    our_node: String,
    send_to_loop: MessageSender,
    send_to_terminal: PrintSender,
    mut recv_from_loop: MessageReceiver,
    send_to_caps_oracle: CapMessageSender,
) -> anyhow::Result<()> {
    let mut drive_to_vfs: IdentifierToVfs = HashMap::new();
    let mut response_router: ResponseRouter = HashMap::new();
    let (send_vfs_task_done, mut recv_vfs_task_done): (
        tokio::sync::mpsc::Sender<u64>,
        tokio::sync::mpsc::Receiver<u64>,
    ) = tokio::sync::mpsc::channel(VFS_TASK_DONE_CHANNEL_CAPACITY);
    let (send_persist_state, mut recv_persist_state): (
        tokio::sync::mpsc::Sender<bool>,
        tokio::sync::mpsc::Receiver<bool>,
    ) = tokio::sync::mpsc::channel(VFS_PERSIST_STATE_CHANNEL_CAPACITY);

    let (response_sender, response_receiver): (MessageSender, MessageReceiver) =
        tokio::sync::mpsc::channel(VFS_RESPONSE_CHANNEL_CAPACITY);
    let first_message_id = rand::random();
    response_router.insert(first_message_id, response_sender);
    let is_reboot = load_state_from_reboot(
        our_node.clone(),
        &send_to_loop,
        response_receiver,
        &mut drive_to_vfs,
        first_message_id,
    )
    .await;
    if !is_reboot {
        // initial boot
        // build_state_for_initial_boot(&process_map, &mut drive_to_vfs);
        send_persist_state.send(true).await.unwrap();
    }

    loop {
        tokio::select! {
            id_done = recv_vfs_task_done.recv() => {
                let Some(id_done) = id_done else { continue };
                response_router.remove(&id_done);
            },
            _ = recv_persist_state.recv() => {
                persist_state(our_node.clone(), &send_to_loop, &drive_to_vfs).await;
                continue;
            },
            km = recv_from_loop.recv() => {
                let Some(km) = km else { continue };
                if let Some(response_sender) = response_router.remove(&km.id) {
                    response_sender.send(km).await.unwrap();
                    continue;
                }

                let KernelMessage {
                    id,
                    source,
                    rsvp,
                    message,
                    payload,
                    ..
                } = km;
                let Message::Request(Request {
                    expects_response,
                    ipc: Some(ipc),
                    metadata, // we return this to Requester for kernel reasons
                    ..
                }) = message.clone()
                else {
                    // consider moving this handling into it's own function
                    continue;
                };

                let request: VfsRequest = match serde_json::from_str(&ipc) {
                    Ok(r) => r,
                    Err(e) => {
                        println!("vfs: got invalid Request: {}", e);
                        continue;
                    }
                };

                println!("vfs: got request: {:?}\r", request);

                if our_node != source.node {
                    println!(
                        "vfs: request must come from our_node={}, got: {}",
                        our_node,
                        source.node,
                    );
                    continue;
                }

                let (vfs, new_caps) = match drive_to_vfs.get(&request.drive) {
                    Some(vfs) => (Arc::clone(vfs), vec![]),
                    None => {
                        let VfsAction::New = request.action else {
                            println!("vfs: invalid Request: non-New to non-existent: {:?}\r", request);
                            send_to_loop
                                .send(make_error_message(
                                    our_node.clone(),
                                    id,
                                    source.clone(),
                                    VfsError::BadDriveName,
                                ))
                                .await
                                .unwrap();
                            continue;
                        };
                        drive_to_vfs.insert(
                            request.drive.clone(),
                            Arc::new(Mutex::new(Vfs::new())),
                        );
                        let read_cap = Capability {
                            issuer: Address {
                                node: our_node.clone(),
                                process: VFS_PROCESS_ID.clone(),
                            },
                            params: serde_json::to_string(
                                &serde_json::json!({"kind": "read", "drive": request.drive})
                            ).unwrap(),
                        };
                        let write_cap = Capability {
                            issuer: Address {
                                node: our_node.clone(),
                                process: VFS_PROCESS_ID.clone(),
                            },
                            params: serde_json::to_string(
                                &serde_json::json!({"kind": "write", "drive": request.drive})
                            ).unwrap(),
                        };
                        (
                            Arc::clone(drive_to_vfs.get(&request.drive).unwrap()),
                            vec![read_cap, write_cap],
                        )
                    }
                };

                //  TODO: remove after vfs is stable
                let _ = send_to_terminal.send(Printout {
                    verbosity: 1,
                    content: format!("{:?}", vfs)
                }).await;

                let (response_sender, response_receiver): (
                    MessageSender,
                    MessageReceiver,
                ) = tokio::sync::mpsc::channel(VFS_RESPONSE_CHANNEL_CAPACITY);
                response_router.insert(id.clone(), response_sender);
                let our_node = our_node.clone();
                let send_to_loop = send_to_loop.clone();
                let send_persist_state = send_persist_state.clone();
                let send_to_terminal = send_to_terminal.clone();
                let send_to_caps_oracle = send_to_caps_oracle.clone();
                let send_vfs_task_done = send_vfs_task_done.clone();
                match &message {
                    Message::Response(_) => {},
                    Message::Request(_) => {
                        tokio::spawn(async move {
                            match handle_request(
                                our_node.clone(),
                                id,
                                source.clone(),
                                expects_response,
                                rsvp,
                                request,
                                metadata,
                                payload,
                                new_caps,
                                vfs,
                                send_to_loop.clone(),
                                send_persist_state,
                                send_to_terminal,
                                send_to_caps_oracle,
                                response_receiver,
                            ).await {
                                Err(e) => {
                                    send_to_loop
                                        .send(make_error_message(
                                            our_node.into(),
                                            id,
                                            source,
                                            e,
                                        ))
                                        .await
                                        .unwrap();
                                },
                                Ok(_) => {},
                            }
                            send_vfs_task_done.send(id).await.unwrap();
                        });
                    },
                }
            },
        }
    }
}

//  TODO: error handling: send error messages to caller
async fn handle_request(
    our_node: String,
    id: u64,
    source: Address,
    expects_response: Option<u64>,
    rsvp: Rsvp,
    request: VfsRequest,
    metadata: Option<String>,
    payload: Option<Payload>,
    new_caps: Vec<Capability>,
    vfs: Arc<Mutex<Vfs>>,
    send_to_loop: MessageSender,
    send_to_persist: tokio::sync::mpsc::Sender<bool>,
    send_to_terminal: PrintSender,
    send_to_caps_oracle: CapMessageSender,
    recv_response: MessageReceiver,
) -> Result<(), VfsError> {
    let (send_cap_bool, recv_cap_bool) = tokio::sync::oneshot::channel();
    match &request.action {
        VfsAction::Add { .. }
        | VfsAction::Rename { .. }
        | VfsAction::Delete { .. }
        | VfsAction::WriteOffset { .. }
        | VfsAction::SetSize { .. } => {
            let _ = send_to_caps_oracle
                .send(CapMessage::Has {
                    on: source.process.clone(),
                    cap: Capability {
                        issuer: Address {
                            node: our_node.clone(),
                            process: VFS_PROCESS_ID.clone(),
                        },
                        params: serde_json::to_string(&serde_json::json!({
                            "kind": "write",
                            "drive": request.drive,
                        }))
                        .unwrap(),
                    },
                    responder: send_cap_bool,
                })
                .unwrap();
            let has_cap = recv_cap_bool.await.unwrap();
            if !has_cap {
                return Err(VfsError::NoCap);
            }
        }
        VfsAction::GetPath { .. }
        | VfsAction::GetHash { .. }
        | VfsAction::GetEntry { .. }
        | VfsAction::GetFileChunk { .. }
        | VfsAction::GetEntryLength { .. } => {
            let _ = send_to_caps_oracle
                .send(CapMessage::Has {
                    on: source.process.clone(),
                    cap: Capability {
                        issuer: Address {
                            node: our_node.clone(),
                            process: VFS_PROCESS_ID.clone(),
                        },
                        params: serde_json::to_string(&serde_json::json!({
                            "kind": "read",
                            "drive": request.drive,
                        }))
                        .unwrap(),
                    },
                    responder: send_cap_bool,
                })
                .unwrap();
            let has_cap = recv_cap_bool.await.unwrap();
            if !has_cap {
                return Err(VfsError::NoCap);
            }
        }
        _ => {} // New
    }

    let (ipc, bytes) = match_request(
        our_node.clone(),
        id.clone(),
        source.clone(),
        request,
        payload,
        new_caps,
        vfs,
        &send_to_loop,
        &send_to_persist,
        &send_to_terminal,
        recv_response,
    )
    .await?;

    //  TODO: properly handle rsvp
    if expects_response.is_some() {
        let response = KernelMessage {
            id,
            source: Address {
                node: our_node.clone(),
                process: VFS_PROCESS_ID.clone(),
            },
            target: Address {
                node: our_node.clone(),
                process: source.process.clone(),
            },
            rsvp,
            message: Message::Response((Response { ipc, metadata }, None)),
            payload: match bytes {
                Some(bytes) => Some(Payload {
                    mime: Some("application/octet-stream".into()),
                    bytes,
                }),
                None => None,
            },
            signed_capabilities: None,
        };

        let _ = send_to_loop.send(response).await;
    }

    Ok(())
}

// #[async_recursion::async_recursion]
async fn match_request(
    our_node: String,
    id: u64,
    source: Address,
    request: VfsRequest,
    payload: Option<Payload>,
    new_caps: Vec<Capability>,
    vfs: Arc<Mutex<Vfs>>,
    send_to_loop: &MessageSender,
    send_to_persist: &tokio::sync::mpsc::Sender<bool>,
    send_to_terminal: &PrintSender,
    mut recv_response: MessageReceiver,
) -> Result<(Option<String>, Option<Vec<u8>>), VfsError> {
    Ok(match request.action {
        VfsAction::New => {
            for new_cap in new_caps {
                let _ = send_to_loop
                    .send(KernelMessage {
                        id,
                        source: Address {
                            node: our_node.clone(),
                            process: VFS_PROCESS_ID.clone(),
                        },
                        target: Address {
                            node: our_node.clone(),
                            process: ProcessId::new(Some("kernel"), "sys", "uqbar"),
                        },
                        rsvp: None,
                        message: Message::Request(Request {
                            inherit: false,
                            expects_response: None,
                            ipc: Some(
                                serde_json::to_string(&KernelCommand::GrantCapability {
                                    to_process: source.process.clone(),
                                    params: new_cap.params,
                                })
                                .unwrap(),
                            ),
                            metadata: None,
                        }),
                        payload: None,
                        signed_capabilities: None,
                    })
                    .await;
            }
            send_to_persist.send(true).await.unwrap();
            (Some(serde_json::to_string(&VfsResponse::Ok).unwrap()), None)
        }
        VfsAction::Add {
            full_path,
            entry_type,
        } => {
            match entry_type {
                AddEntryType::Dir => {
                    if let Some(last_char) = full_path.chars().last() {
                        if last_char != '/' {
                            //  TODO: panic or correct & notify?
                            //  elsewhere we panic
                            // format!("{}/", full_path)
                            send_to_terminal
                                .send(Printout {
                                    verbosity: 0,
                                    content: format!(
                                        "vfs: cannot add dir without trailing `/`: {}",
                                        full_path
                                    ),
                                })
                                .await
                                .unwrap();
                            panic!("");
                        };
                    } else {
                        panic!("empty path");
                    };
                    let mut vfs = vfs.lock().await;
                    if vfs.path_to_key.contains_key(&full_path) {
                        send_to_terminal
                            .send(Printout {
                                verbosity: 0,
                                content: format!("vfs: not overwriting dir {}", full_path),
                            })
                            .await
                            .unwrap();
                        panic!(""); //  TODO: error?
                    };
                    let (name, parent_path) = make_dir_name(&full_path);
                    let Some(parent_key) = vfs.path_to_key.remove(&parent_path) else {
                        panic!("fp, pp: {}, {}", full_path, parent_path);
                    };
                    let key = Key::Dir { id: rand::random() };
                    vfs.key_to_entry.insert(
                        key.clone(),
                        Entry {
                            name,
                            full_path: full_path.clone(),
                            entry_type: EntryType::Dir {
                                parent: parent_key.clone(),
                                children: HashSet::new(),
                            },
                        },
                    );
                    vfs.path_to_key.insert(parent_path, parent_key);
                    vfs.path_to_key.insert(full_path.clone(), key.clone());
                }
                AddEntryType::NewFile => {
                    if let Some(last_char) = full_path.chars().last() {
                        if last_char == '/' {
                            send_to_terminal
                                .send(Printout {
                                    verbosity: 0,
                                    content: format!(
                                        "vfs: file path cannot end with `/`: {}",
                                        full_path,
                                    ),
                                })
                                .await
                                .unwrap();
                            panic!("");
                        }
                    } else {
                        panic!("empty path");
                    };
                    let mut vfs = vfs.lock().await;
                    if vfs.path_to_key.contains_key(&full_path) {
                        send_to_terminal
                            .send(Printout {
                                verbosity: 1,
                                content: format!("vfs: overwriting file {}", full_path),
                            })
                            .await
                            .unwrap();
                        let Some(old_key) = vfs.path_to_key.remove(&full_path) else {
                            panic!("");
                        };
                        vfs.key_to_entry.remove(&old_key);
                    };

                    let _ = send_to_loop
                        .send(KernelMessage {
                            id,
                            source: Address {
                                node: our_node.clone(),
                                process: VFS_PROCESS_ID.clone(),
                            },
                            target: Address {
                                node: our_node.clone(),
                                process: FILESYSTEM_PROCESS_ID.clone(),
                            },
                            rsvp: None,
                            message: Message::Request(Request {
                                inherit: true,
                                expects_response: Some(5), // TODO evaluate
                                ipc: Some(serde_json::to_string(&FsAction::Write).unwrap()),
                                metadata: None,
                            }),
                            payload,
                            signed_capabilities: None,
                        })
                        .await;
                    let write_response = recv_response.recv().await.unwrap();
                    let KernelMessage { message, .. } = write_response;
                    let Message::Response((Response { ipc, .. }, None)) = message else {
                        panic!("");
                    };
                    let Some(ipc) = ipc else {
                        panic!("");
                    };
                    let Ok(FsResponse::Write(hash)) =
                        serde_json::from_str::<Result<FsResponse, FsError>>(&ipc).unwrap()
                    else {
                        panic!("");
                    };

                    let (name, parent_path) = make_file_name(&full_path);
                    let Some(parent_key) = vfs.path_to_key.remove(&parent_path) else {
                        panic!("");
                    };
                    let key = Key::File { id: hash };
                    vfs.key_to_entry.insert(
                        key.clone(),
                        Entry {
                            name,
                            full_path: full_path.clone(),
                            entry_type: EntryType::File {
                                parent: parent_key.clone(),
                            },
                        },
                    );
                    vfs.path_to_key.insert(parent_path, parent_key);
                    vfs.path_to_key.insert(full_path.clone(), key.clone());
                }
                AddEntryType::ExistingFile { hash } => {
                    if let Some(last_char) = full_path.chars().last() {
                        if last_char == '/' {
                            send_to_terminal
                                .send(Printout {
                                    verbosity: 0,
                                    content: format!(
                                        "vfs: file path cannot end with `/`: {}",
                                        full_path,
                                    ),
                                })
                                .await
                                .unwrap();
                            panic!("");
                        }
                    } else {
                        panic!("empty path");
                    };
                    let mut vfs = vfs.lock().await;
                    if vfs.path_to_key.contains_key(&full_path) {
                        send_to_terminal
                            .send(Printout {
                                verbosity: 1,
                                content: format!("vfs: overwriting file {}", full_path),
                            })
                            .await
                            .unwrap();
                        let Some(old_key) = vfs.path_to_key.remove(&full_path) else {
                            panic!("no old key");
                        };
                        vfs.key_to_entry.remove(&old_key);
                    };
                    let (name, parent_path) = make_file_name(&full_path);
                    let Some(parent_key) = vfs.path_to_key.remove(&parent_path) else {
                        panic!("");
                    };
                    let key = Key::File { id: hash };
                    vfs.key_to_entry.insert(
                        key.clone(),
                        Entry {
                            name,
                            full_path: full_path.clone(),
                            entry_type: EntryType::File {
                                parent: parent_key.clone(),
                            },
                        },
                    );
                    vfs.path_to_key.insert(parent_path, parent_key);
                    vfs.path_to_key.insert(full_path.clone(), key.clone());
                }
                AddEntryType::ZipArchive => {
                    let Some(payload) = payload else {
                        panic!("");
                    };
                    let Some(mime) = payload.mime else {
                        panic!("");
                    };
                    if "application/zip" != mime {
                        panic!("");
                    }
                    let file = std::io::Cursor::new(&payload.bytes);
                    let mut zip = match zip::ZipArchive::new(file) {
                        Ok(f) => f,
                        Err(e) => panic!("vfs: zip error: {:?}", e),
                    };

                    // loop through items in archive; recursively add to root
                    for i in 0..zip.len() {
                        // must destruct the zip file created in zip.by_index()
                        //  Before any `.await`s are called since ZipFile is not
                        //  Send and so does not play nicely with await
                        let (is_file, is_dir, full_path, file_contents) = {
                            let mut file = zip.by_index(i).unwrap();
                            let is_file = file.is_file();
                            let is_dir = file.is_dir();
                            let full_path = format!("/{}", file.name());
                            let mut file_contents = Vec::new();
                            if is_file {
                                file.read_to_end(&mut file_contents).unwrap();
                            };
                            (is_file, is_dir, full_path, file_contents)
                        };
                        if is_file {
                            let _ = send_to_loop
                                .send(KernelMessage {
                                    id,
                                    source: Address {
                                        node: our_node.clone(),
                                        process: VFS_PROCESS_ID.clone(),
                                    },
                                    target: Address {
                                        node: our_node.clone(),
                                        process: FILESYSTEM_PROCESS_ID.clone(),
                                    },
                                    rsvp: None,
                                    message: Message::Request(Request {
                                        inherit: true,
                                        expects_response: Some(5), // TODO evaluate
                                        ipc: Some(serde_json::to_string(&FsAction::Write).unwrap()),
                                        metadata: None,
                                    }),
                                    payload: Some(Payload {
                                        mime: None,
                                        bytes: file_contents,
                                    }),
                                    signed_capabilities: None,
                                })
                                .await;
                            let write_response = recv_response.recv().await.unwrap();
                            let KernelMessage { message, .. } = write_response;
                            let Message::Response((Response { ipc, metadata: _ }, None)) = message
                            else {
                                panic!("")
                            };
                            let Some(ipc) = ipc else {
                                panic!("");
                            };
                            let FsResponse::Write(hash) = serde_json::from_str(&ipc).unwrap()
                            else {
                                panic!("");
                            };

                            let (name, parent_path) = make_file_name(&full_path);
                            let mut vfs = vfs.lock().await;
                            let Some(parent_key) = vfs.path_to_key.remove(&parent_path) else {
                                panic!("");
                            };
                            let key = Key::File { id: hash };
                            vfs.key_to_entry.insert(
                                key.clone(),
                                Entry {
                                    name,
                                    full_path: full_path.clone(),
                                    entry_type: EntryType::File {
                                        parent: parent_key.clone(),
                                    },
                                },
                            );
                            vfs.path_to_key.insert(parent_path, parent_key);
                            vfs.path_to_key.insert(full_path.clone(), key.clone());
                        } else if is_dir {
                            panic!("vfs: zip dir not yet implemented");
                        } else {
                            panic!("vfs: zip with non-file non-dir");
                        };
                        // if file.is_file() {
                        //     println!("Filename: {}", file.name());
                        //     let mut out = Vec::new();
                        //     file.read_to_end(&mut out).unwrap();
                        //     let full_path = format!("/{}", file.name());

                        //     // TODO: factor out
                        //     let _ = send_to_loop
                        //         .send(KernelMessage {
                        //             id,
                        //             source: Address {
                        //                 node: our_node.clone(),
                        //                 process: VFS_PROCESS_ID.clone(),
                        //             },
                        //             target: Address {
                        //                 node: our_node.clone(),
                        //                 process: FILESYSTEM_PROCESS_ID.clone(),
                        //             },
                        //             rsvp: None,
                        //             message: Message::Request(Request {
                        //                 inherit: true,
                        //                 expects_response: Some(5), // TODO evaluate
                        //                 ipc: Some(serde_json::to_string(&FsAction::Write).unwrap()),
                        //                 metadata: None,
                        //             }),
                        //             payload: Some(Payload {
                        //                 mime: None,
                        //                 bytes: out,
                        //             }),
                        //             signed_capabilities: None,
                        //         })
                        //         .await;
                        //     let write_response = recv_response.recv().await.unwrap();
                        //     let KernelMessage { message, .. } = write_response;
                        //     let Message::Response((Response { ipc, metadata: _ }, None)) = message else {
                        //         panic!("")
                        //     };
                        //     let Some(ipc) = ipc else {
                        //         panic!("");
                        //     };
                        //     let FsResponse::Write(hash) = serde_json::from_str(&ipc).unwrap() else {
                        //         panic!("");
                        //     };

                        //     let (name, parent_path) = make_file_name(&full_path);
                        //     let mut vfs = vfs.lock().await;
                        //     let Some(parent_key) = vfs.path_to_key.remove(&parent_path) else {
                        //         panic!("");
                        //     };
                        //     let key = Key::File { id: hash };
                        //     vfs.key_to_entry.insert(
                        //         key.clone(),
                        //         Entry {
                        //             name,
                        //             full_path: full_path.clone(),
                        //             entry_type: EntryType::File {
                        //                 parent: parent_key.clone(),
                        //             },
                        //         },
                        //     );
                        //     vfs.path_to_key.insert(parent_path, parent_key);
                        //     vfs.path_to_key.insert(full_path.clone(), key.clone());
                        // } else if file.is_dir() {
                        //     panic!("todo");
                        // } else {
                        //     panic!("wat");
                        // }
                    }
                }
            }
            send_to_persist.send(true).await.unwrap();
            (Some(serde_json::to_string(&VfsResponse::Ok).unwrap()), None)
        }
        VfsAction::Rename {
            full_path,
            new_full_path,
        } => {
            let mut vfs = vfs.lock().await;
            let Some(key) = vfs.path_to_key.remove(&full_path) else {
                send_to_terminal
                    .send(Printout {
                        verbosity: 0,
                        content: format!("vfs: can't rename: nonexistent file {}", full_path),
                    })
                    .await
                    .unwrap();
                panic!("");
            };
            let Some(mut entry) = vfs.key_to_entry.remove(&key) else {
                send_to_terminal
                    .send(Printout {
                        verbosity: 0,
                        content: format!("vfs: can't rename: nonexistent file {}", full_path),
                    })
                    .await
                    .unwrap();
                panic!("");
            };
            match entry.entry_type {
                EntryType::Dir { .. } => {
                    if vfs.path_to_key.contains_key(&new_full_path) {
                        send_to_terminal
                            .send(Printout {
                                verbosity: 0,
                                content: format!("vfs: not overwriting dir {}", new_full_path),
                            })
                            .await
                            .unwrap();
                        vfs.path_to_key.insert(full_path, key);
                        panic!(""); //  TODO: error?
                    };
                    let (name, _) = make_dir_name(&new_full_path);
                    entry.name = name;
                    entry.full_path = new_full_path.clone();
                    vfs.path_to_key.insert(new_full_path.clone(), key.clone());
                    vfs.key_to_entry.insert(key, entry);
                    //  TODO: recursively apply path update to all children
                    //  update_child_paths(full_path, new_full_path, children);
                }
                EntryType::File { parent: _ } => {
                    if vfs.path_to_key.contains_key(&new_full_path) {
                        send_to_terminal
                            .send(Printout {
                                verbosity: 1,
                                content: format!("vfs: overwriting file {}", new_full_path),
                            })
                            .await
                            .unwrap();
                    };
                    let (name, _) = make_file_name(&new_full_path);
                    entry.name = name;
                    entry.full_path = new_full_path.clone();
                    vfs.path_to_key.insert(new_full_path.clone(), key.clone());
                    vfs.key_to_entry.insert(key, entry);
                }
            }
            send_to_persist.send(true).await.unwrap();
            (Some(serde_json::to_string(&VfsResponse::Ok).unwrap()), None)
        }
        VfsAction::Delete(full_path) => {
            let mut vfs = vfs.lock().await;
            let Some(key) = vfs.path_to_key.remove(&full_path) else {
                send_to_terminal
                    .send(Printout {
                        verbosity: 0,
                        content: format!("vfs: can't delete: nonexistent entry {}", full_path),
                    })
                    .await
                    .unwrap();
                panic!("");
            };
            let Some(entry) = vfs.key_to_entry.remove(&key) else {
                send_to_terminal
                    .send(Printout {
                        verbosity: 0,
                        content: format!("vfs: can't delete: nonexistent entry {}", full_path),
                    })
                    .await
                    .unwrap();
                panic!("");
            };
            match entry.entry_type {
                EntryType::Dir {
                    parent: _,
                    ref children,
                } => {
                    if !children.is_empty() {
                        send_to_terminal
                            .send(Printout {
                                verbosity: 0,
                                content: format!(
                                    "vfs: can't delete: non-empty directory {}",
                                    full_path
                                ),
                            })
                            .await
                            .unwrap();
                        vfs.path_to_key.insert(full_path.clone(), key.clone());
                        vfs.key_to_entry.insert(key.clone(), entry);
                    }
                }
                EntryType::File { parent } => {
                    match vfs.key_to_entry.get_mut(&parent) {
                        None => {
                            send_to_terminal
                                .send(Printout {
                                    verbosity: 0,
                                    content: format!(
                                        "vfs: delete: unexpected file with no parent dir: {}",
                                        full_path
                                    ),
                                })
                                .await
                                .unwrap();
                            panic!("");
                        }
                        Some(parent) => {
                            let EntryType::Dir {
                                parent: _,
                                ref mut children,
                            } = parent.entry_type
                            else {
                                panic!("");
                            };
                            //  TODO: does this work?
                            children.remove(&key);
                        }
                    }
                }
            }
            send_to_persist.send(true).await.unwrap();
            (Some(serde_json::to_string(&VfsResponse::Ok).unwrap()), None)
        }
        VfsAction::WriteOffset { full_path, offset } => {
            let file_hash = {
                let mut vfs = vfs.lock().await;
                let Some(key) = vfs.path_to_key.remove(&full_path) else {
                    panic!("");
                };
                let key2 = key.clone();
                let Key::File { id: file_hash } = key2 else {
                    panic!(""); //  TODO
                };
                vfs.path_to_key.insert(full_path.clone(), key);
                file_hash
            };
            let _ = send_to_loop
                .send(KernelMessage {
                    id,
                    source: Address {
                        node: our_node.clone(),
                        process: VFS_PROCESS_ID.clone(),
                    },
                    target: Address {
                        node: our_node.clone(),
                        process: FILESYSTEM_PROCESS_ID.clone(),
                    },
                    rsvp: None,
                    message: Message::Request(Request {
                        inherit: true,
                        expects_response: Some(5), // TODO evaluate
                        ipc: Some(
                            serde_json::to_string(&FsAction::WriteOffset((file_hash, offset)))
                                .unwrap(),
                        ),
                        metadata: None,
                    }),
                    payload,
                    signed_capabilities: None,
                })
                .await;

            (Some(serde_json::to_string(&VfsResponse::Ok).unwrap()), None)
        }
        VfsAction::SetSize { full_path, size } => {
            let file_hash = {
                let mut vfs = vfs.lock().await;
                let Some(key) = vfs.path_to_key.remove(&full_path) else {
                    panic!(""); //  TODO
                };
                let key2 = key.clone();
                let Key::File { id: file_hash } = key2 else {
                    panic!(""); //  TODO
                };
                vfs.path_to_key.insert(full_path.clone(), key);
                file_hash
            };

            let _ = send_to_loop
                .send(KernelMessage {
                    id,
                    source: Address {
                        node: our_node.clone(),
                        process: VFS_PROCESS_ID.clone(),
                    },
                    target: Address {
                        node: our_node.clone(),
                        process: FILESYSTEM_PROCESS_ID.clone(),
                    },
                    rsvp: None,
                    message: Message::Request(Request {
                        inherit: true,
                        expects_response: Some(15),
                        ipc: Some(
                            serde_json::to_string(&FsAction::SetLength((file_hash.clone(), size)))
                                .unwrap(),
                        ),
                        metadata: None,
                    }),
                    payload: None,
                    signed_capabilities: None,
                })
                .await;
            let read_response = recv_response.recv().await.unwrap();
            let KernelMessage { message, .. } = read_response;
            let Message::Response((Response { ipc, metadata: _ }, None)) = message else {
                panic!("")
            };
            let Some(ipc) = ipc else {
                panic!("");
            };
            let FsResponse::Length(length) = serde_json::from_str(&ipc).unwrap() else {
                panic!("");
            };
            assert_eq!(size, length);
            // let Some(payload) = payload else {
            //     panic!("");
            // };
            (Some(serde_json::to_string(&VfsResponse::Ok).unwrap()), None)
        }
        VfsAction::GetPath(hash) => {
            let mut vfs = vfs.lock().await;
            let key = Key::File { id: hash.clone() };
            let ipc = Some(
                serde_json::to_string(&VfsResponse::GetPath(match vfs.key_to_entry.remove(&key) {
                    None => None,
                    Some(entry) => {
                        let full_path = entry.full_path.clone();
                        vfs.key_to_entry.insert(key, entry);
                        Some(full_path)
                    }
                }))
                .unwrap(),
            );
            (ipc, None)
        }
        VfsAction::GetHash(full_path) => {
            let vfs = vfs.lock().await;
            let mut ipc = Some(serde_json::to_string(&VfsResponse::GetHash(None)).unwrap());
            if let Some(key) = vfs.path_to_key.get(&full_path) {
                if let Key::File { id: hash } = key {
                    ipc = Some(serde_json::to_string(&VfsResponse::GetHash(Some(*hash))).unwrap());
                };
            }
            (ipc, None)
        }
        VfsAction::GetEntry(ref full_path) => {
            let (key, entry, paths) = {
                let mut vfs = vfs.lock().await;
                let key = vfs.path_to_key.remove(full_path);
                match key {
                    None => (None, None, vec![]),
                    Some(key) => {
                        vfs.path_to_key.insert(full_path.clone(), key.clone());
                        let entry = vfs.key_to_entry.remove(&key);
                        match entry {
                            None => (Some(key), None, vec![]),
                            Some(ref e) => {
                                vfs.key_to_entry.insert(key.clone(), e.clone());
                                match e.entry_type {
                                    EntryType::File { parent: _ } => (Some(key), entry, vec![]),
                                    EntryType::Dir {
                                        parent: _,
                                        ref children,
                                    } => {
                                        let mut paths: Vec<String> = Vec::new();
                                        for child in children {
                                            let Some(child) = vfs.key_to_entry.get(&child) else {
                                                send_to_terminal
                                                    .send(Printout {
                                                        verbosity: 0,
                                                        content: format!(
                                                            "vfs: child missing for: {}",
                                                            full_path
                                                        ),
                                                    })
                                                    .await
                                                    .unwrap();
                                                continue;
                                            };
                                            paths.push(child.full_path.clone());
                                        }
                                        paths.sort();
                                        (Some(key), entry, paths)
                                    }
                                }
                            }
                        }
                    }
                }
            };

            let entry_not_found = (
                Some(
                    serde_json::to_string(&VfsResponse::GetEntry {
                        exists: false,
                        children: vec![],
                    })
                    .unwrap(),
                ),
                None,
            );
            match key {
                None => entry_not_found,
                Some(key) => match entry {
                    None => entry_not_found,
                    Some(entry) => match entry.entry_type {
                        EntryType::Dir {
                            parent: _,
                            children: _,
                        } => (
                            Some(
                                serde_json::to_string(&VfsResponse::GetEntry {
                                    exists: true,
                                    children: paths,
                                })
                                .unwrap(),
                            ),
                            None,
                        ),
                        EntryType::File { parent: _ } => {
                            let Key::File { id: file_hash } = key else {
                                panic!("");
                            };
                            let _ = send_to_loop
                                .send(KernelMessage {
                                    id,
                                    source: Address {
                                        node: our_node.clone(),
                                        process: VFS_PROCESS_ID.clone(),
                                    },
                                    target: Address {
                                        node: our_node.clone(),
                                        process: FILESYSTEM_PROCESS_ID.clone(),
                                    },
                                    rsvp: None,
                                    message: Message::Request(Request {
                                        inherit: true,
                                        expects_response: Some(5), // TODO evaluate
                                        ipc: Some(
                                            serde_json::to_string(&FsAction::Read(
                                                file_hash.clone(),
                                            ))
                                            .unwrap(),
                                        ),
                                        metadata: None,
                                    }),
                                    payload: None,
                                    signed_capabilities: None,
                                })
                                .await;
                            let read_response = recv_response.recv().await.unwrap();
                            let KernelMessage {
                                message, payload, ..
                            } = read_response;
                            let Message::Response((Response { ipc, metadata: _ }, None)) = message
                            else {
                                panic!("");
                            };
                            let Some(ipc) = ipc else {
                                panic!("");
                            };
                            let Ok(FsResponse::Read(read_hash)) =
                                serde_json::from_str::<Result<FsResponse, FsError>>(&ipc).unwrap()
                            else {
                                panic!("");
                            };
                            // TODO get rid of PANICS!
                            assert_eq!(file_hash, read_hash);
                            let Some(payload) = payload else {
                                panic!("");
                            };
                            (
                                Some(
                                    serde_json::to_string(&VfsResponse::GetEntry {
                                        exists: true,
                                        children: vec![],
                                    })
                                    .unwrap(),
                                ),
                                Some(payload.bytes),
                            )
                        }
                    },
                },
            }
        }
        VfsAction::GetFileChunk {
            ref full_path,
            offset,
            length,
        } => {
            let file_hash = {
                let mut vfs = vfs.lock().await;
                let Some(key) = vfs.path_to_key.remove(full_path) else {
                    panic!(""); //  TODO
                };
                let key2 = key.clone();
                let Key::File { id: file_hash } = key2 else {
                    panic!(""); //  TODO
                };
                vfs.path_to_key.insert(full_path.clone(), key);
                file_hash
            };

            let _ = send_to_loop
                .send(KernelMessage {
                    id,
                    source: Address {
                        node: our_node.clone(),
                        process: VFS_PROCESS_ID.clone(),
                    },
                    target: Address {
                        node: our_node.clone(),
                        process: FILESYSTEM_PROCESS_ID.clone(),
                    },
                    rsvp: None,
                    message: Message::Request(Request {
                        inherit: true,
                        expects_response: Some(5), // TODO evaluate
                        ipc: Some(
                            serde_json::to_string(&FsAction::ReadChunk(ReadChunkRequest {
                                file: file_hash.clone(),
                                start: offset,
                                length,
                            }))
                            .unwrap(),
                        ),
                        metadata: None,
                    }),
                    payload: None,
                    signed_capabilities: None,
                })
                .await;
            let read_response = recv_response.recv().await.unwrap();
            let KernelMessage {
                message, payload, ..
            } = read_response;
            let Message::Response((Response { ipc, metadata: _ }, None)) = message else {
                panic!("")
            };
            let Some(ipc) = ipc else {
                panic!("");
            };
            let Ok(FsResponse::ReadChunk(read_hash)) =
                serde_json::from_str::<Result<FsResponse, FsError>>(&ipc).unwrap()
            else {
                panic!("");
            };
            assert_eq!(file_hash, read_hash);
            let Some(payload) = payload else {
                panic!("");
            };

            (
                Some(serde_json::to_string(&VfsResponse::GetFileChunk).unwrap()),
                Some(payload.bytes),
            )
        }
        VfsAction::GetEntryLength(ref full_path) => {
            if full_path.chars().last() == Some('/') {
                (
                    Some(serde_json::to_string(&VfsResponse::GetEntryLength(None)).unwrap()),
                    None,
                )
            } else {
                let file_hash = {
                    let mut vfs = vfs.lock().await;
                    let Some(key) = vfs.path_to_key.remove(full_path) else {
                        panic!("");
                    };
                    let key2 = key.clone();
                    let Key::File { id: file_hash } = key2 else {
                        panic!(""); //  TODO
                    };
                    vfs.path_to_key.insert(full_path.clone(), key);
                    file_hash
                };

                let _ = send_to_loop
                    .send(KernelMessage {
                        id,
                        source: Address {
                            node: our_node.clone(),
                            process: VFS_PROCESS_ID.clone(),
                        },
                        target: Address {
                            node: our_node.clone(),
                            process: FILESYSTEM_PROCESS_ID.clone(),
                        },
                        rsvp: None,
                        message: Message::Request(Request {
                            inherit: true,
                            expects_response: Some(5), // TODO evaluate
                            ipc: Some(serde_json::to_string(&FsAction::Length(file_hash)).unwrap()),
                            metadata: None,
                        }),
                        payload: None,
                        signed_capabilities: None,
                    })
                    .await;
                let length_response = recv_response.recv().await.unwrap();
                let KernelMessage { message, .. } = length_response;
                let Message::Response((Response { ipc, metadata: _ }, None)) = message else {
                    panic!("")
                };
                let Some(ipc) = ipc else {
                    panic!("");
                };
                let Ok(FsResponse::Length(length)) =
                    serde_json::from_str::<Result<FsResponse, FsError>>(&ipc).unwrap()
                else {
                    panic!("");
                };

                (
                    Some(
                        serde_json::to_string(&VfsResponse::GetEntryLength(Some(length))).unwrap(),
                    ),
                    None,
                )
            }
        }
    })
}
