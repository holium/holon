// This is an ERC20 Manager app meant to show how to interact with an ERC20 contract
// using alloy and uqbar.
// TODO parse logs to create some sort of state

cargo_component_bindings::generate!();

use bindings::component::microkernel_process::types;
use serde_json::json;
use alloy_primitives::{address, Address, U256};
use alloy_sol_types::{sol, SolEnum, SolType, SolCall};
use serde::{Deserialize, Serialize};
use hex;

mod process_lib;

const ERC20_COMPILED: &str = include_str!("TestERC20.json");

struct Component;

// examples
// !message tuna eth_demo {"token":"5fbdb2315678afecb367f032d93f642f64180aa3", "method": "TotalSupply"}
// !message tuna eth_demo {"token":"5fbdb2315678afecb367f032d93f642f64180aa3", "method":{"BalanceOf":"f39fd6e51aad88f6f4ce6ab8827279cfffb92266"}}
// !message tuna eth_demo {"token":"5fbdb2315678afecb367f032d93f642f64180aa3", "method":{"BalanceOf":"8bbe911710c9e592487dde0735db76f83dc44cfd"}}
// !message tuna eth_demo {"token":"5fbdb2315678afecb367f032d93f642f64180aa3", "method":{"Transfer":{"recipient": "8bbe911710c9e592487dde0735db76f83dc44cfd","amount":"fff"}}}
// !message tuna eth_demo {"token":"5fbdb2315678afecb367f032d93f642f64180aa3", "method":{"Approve":{"spender": "8bbe911710c9e592487dde0735db76f83dc44cfd","amount":"fff"}}}
// !message tuna eth_demo {"token":"5fbdb2315678afecb367f032d93f642f64180aa3", "method":{"TransferFrom":{"sender": "8bbe911710c9e592487dde0735db76f83dc44cfd","recipient":"8bbe911710c9e592487dde0735db76f83dc44cfd","amount":"fff"}}}

#[derive(Debug, Serialize, Deserialize)]
struct Erc20Action {
    token: String,
    method: Erc20Method,
}

#[derive(Debug, Serialize, Deserialize)]
enum Erc20Method {
    // views
    TotalSupply,
    BalanceOf(String),
    // writes
    Transfer(Transfer),
    Approve(Approve),
    TransferFrom(TransferFrom),
}

#[derive(Debug, Serialize, Deserialize)]
struct Transfer {
    recipient: String, // no 0x prefix on any of these types
    amount: String, // hex encoded
}

#[derive(Debug, Serialize, Deserialize)]
struct Approve {
    spender: String,
    amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransferFrom {
    sender: String,
    recipient: String,
    amount: String,
}

sol! {
    function totalSupply() external view returns (uint256);
    function balanceOf(address account) external view returns (uint256);
    function transfer(address recipient, uint256 amount) external returns (bool);
    function allowance(address owner, address spender) external view returns (uint256);
    function approve(address spender, uint256 amount) external returns (bool);
    function transferFrom(address sender, address recipient, uint256 amount) external returns (bool);
}

impl bindings::MicrokernelProcess for Component {
    fn run_process(our: String, dap: String) {
        bindings::print_to_terminal(0, "eth-demo: start");

        let compiled: serde_json::Value =
            serde_json::from_str(ERC20_COMPILED).unwrap();

        let bc: Vec<u8> = decode_hex(compiled["bytecode"].as_str().unwrap()).unwrap();

        let deployment_res = process_lib::send_request_and_await_response(
            our.clone(),
            "eth_rpc".to_string(),
            Some(json!("DeployContract")),
            types::WitPayloadBytes {
                circumvent: types::WitCircumvent::False,
                content: Some(bc.into())
            },
        );

        bindings::print_to_terminal(0, format!("ERC20 address: {:?}", hex::encode(deployment_res.unwrap().content.payload.bytes.content.unwrap())).as_str());

        loop {
            let (message, _) = bindings::await_next_message().unwrap();  //  TODO: handle error properly
            let Some(message_from_loop_string) = message.content.payload.json else {
                panic!("eth demo requires json payload")
            };

            let message_from_loop: Erc20Action = serde_json::from_str(message_from_loop_string.as_str()).unwrap();

            let (json, call_data): (serde_json::Value, Vec<u8>) = match message_from_loop.method {
                // views
                //
                Erc20Method::TotalSupply => {
                    (
                        json!({"Call": {
                            "contract_address": message_from_loop.token,
                            "gas": null,
                            "gas_price": null,
                        }}),
                        totalSupplyCall{}.encode()
                    )
                },
                Erc20Method::BalanceOf(addr) => {
                    let adr: Address = addr.as_str().parse().unwrap();
                    (
                        json!({"Call": {
                            "contract_address": message_from_loop.token,
                            "gas": null,
                            "gas_price": null,
                        }}),
                        balanceOfCall{
                            account: adr
                        }.encode()
                    )
                },
                // writes
                //
                Erc20Method::Transfer(transfer) => {
                    (
                        json!({"SendTransaction": {
                            "contract_address": message_from_loop.token,
                            "gas": null,
                            "gas_price": null,
                        }}),
                        transferCall{
                            recipient: transfer.recipient.as_str().parse().unwrap(),
                            amount: U256::from_str_radix(&transfer.amount, 16).unwrap()
                        }.encode()
                    )
                },
                Erc20Method::Approve(approve) => {
                    (
                        json!({"SendTransaction": {
                            "contract_address": message_from_loop.token,
                            "gas": null,
                            "gas_price": null,
                        }}),
                        approveCall{
                            spender: approve.spender.as_str().parse().unwrap(),
                            amount: U256::from_str_radix(&approve.amount, 16).unwrap()
                        }.encode()
                    )
                },
                Erc20Method::TransferFrom(transfer_from) => {
                    (
                        json!({"SendTransaction": {
                            "contract_address": message_from_loop.token,
                            "gas": null,
                            "gas_price": null,
                        }}),
                        transferFromCall{
                            sender: transfer_from.sender.as_str().parse().unwrap(),
                            recipient: transfer_from.recipient.as_str().parse().unwrap(),
                            amount: U256::from_str_radix(&transfer_from.amount, 16).unwrap()
                        }.encode()
                    )
                },
            };
            bindings::print_to_terminal(0, format!("call_data: {:?}", call_data).as_str());
            let res = process_lib::send_request_and_await_response(
                our.clone(),
                "eth_rpc".to_string(),
                Some(json),
                types::WitPayloadBytes {
                    circumvent: types::WitCircumvent::False,
                    content: Some(call_data)
                },
            );
            bindings::print_to_terminal(0, format!("response: {:?}", res).as_str());
        }
    }
}


// helpers
fn decode_hex(s: &str) -> Result<Vec<u8>, hex::FromHexError> {
    // If the string starts with "0x", skip the prefix
    let hex_part = if s.starts_with("0x") {
        &s[2..]
    } else {
        s
    };
    hex::decode(hex_part)
}
