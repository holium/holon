use tokio::sync::mpsc;
use std::env;

use ethers::prelude::*;

use crate::types::*;

mod types;
mod terminal;
mod websockets;
mod microkernel;
mod blockchain;
mod engine;

const EVENT_LOOP_CHANNEL_CAPACITY: usize = 10_000;
const TERMINAL_CHANNEL_CAPACITY: usize = 32;
const WEBSOCKET_SENDER_CHANNEL_CAPACITY: usize = 100;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let our_address: H256 = args[1].clone().parse().unwrap();

    // kernel receives system cards via this channel, all other modules send cards
    let (kernel_card_sender, kernel_card_receiver): (CardSender, CardReceiver) = mpsc::channel(EVENT_LOOP_CHANNEL_CAPACITY);
    // websocket sender receives send cards via this channel, kernel send cards
    let (wss_card_sender, wss_card_receiver): (CardSender, CardReceiver) = mpsc::channel(WEBSOCKET_SENDER_CHANNEL_CAPACITY);
    // terminal receives prints via this channel, all other modules send prints
    let (print_sender, print_receiver): (PrintSender, PrintReceiver) = mpsc::channel(TERMINAL_CHANNEL_CAPACITY);

    let uqchain = engine::UqChain::new();
    let my_txn: engine::Transaction = engine::Transaction {
                                        from: our_address,
                                        signature: None,
                                        to: "0x0000000000000000000000000000000000000000000000000000000000005678".parse().unwrap(),
                                        town_id: 0,
                                        calldata: serde_json::to_value("hi").unwrap(),
                                        nonce: U256::from(1),
                                        gas_price: U256::from(0),
                                        gas_limit: U256::from(0),
                                    };
    let _ = uqchain.run_batch(vec![my_txn]);

    // this will be replaced with actual chain reading from indexer module?
    let blockchain = std::fs::File::open("blockchain.json")
        .expect("couldn't read from the chain lolz");
    let json: serde_json::Value = serde_json::from_reader(blockchain)
        .expect("blockchain.json should be proper JSON");
    let pki: OnchainPKI = serde_json::from_value::<OnchainPKI>(json)
        .expect("should be a JSON map of identities");
    // our identity in the uqbar PKI
    let our = pki.get(&our_address).expect("we should be in the PKI").clone();

    /*  we are currently running 2 I/O modules: terminal, websocket
     *  the kernel module will handle our userspace processes and receives
     *  all "cards", the basic message format for uqbar.
     *
     *  future modules: UDP I/O, filesystem, ..?
     *
     *  if any of these modules fail, the program exits with an error.
     */
    let quit: String = tokio::select! {
        term = terminal::terminal(&our, kernel_card_sender.clone(), print_receiver) => match term {
            Ok(_) => "graceful shutdown".to_string(),
            Err(e) => format!("exiting with error: {:?}", e),
        },
        _ = microkernel::kernel(&our, kernel_card_sender.clone(), print_sender.clone(), kernel_card_receiver, wss_card_sender.clone()) => {
            "microkernel died".to_string()
        },
        _ = websockets::websockets(&our, &pki, wss_card_receiver, kernel_card_sender, print_sender) => {
            "websocket sender died".to_string()
        }
    };

    println!("{}", quit);
}
