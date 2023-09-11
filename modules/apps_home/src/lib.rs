cargo_component_bindings::generate!();

use bindings::{print_to_terminal, receive, send_request, send_response, get_payload, Guest};
use bindings::component::uq_process::types::*;
use serde_json::json;

mod process_lib;

struct Component;

const APPS_HOME_PAGE: &str = include_str!("home.html");

impl Guest for Component {
    fn init(our: Address) {
        print_to_terminal(1, "apps_home: start");

        send_request(
            &Address {
                node: our.node.clone(),
                process: ProcessId::Name("http_bindings".to_string()),
            },
            &Request {
                inherit: false,
                expects_response: false,
                ipc: Some(serde_json::json!({
                    "action": "bind-app",
                    "path": "/",
                    "app": "apps_home",
                    "authenticated": true,
                }).to_string()),
                metadata: None,
            },
            None,
            None
        );

        loop {
            let Ok((_source, message)) = receive() else {
                print_to_terminal(0, "apps_home: got network error");
                continue;
            };
            let Message::Request(request) = message else {
                print_to_terminal(0, "apps_home: got unexpected message");
                continue;
            };

            if let Some(json) = request.ipc {
                print_to_terminal(1, format!("apps_home: JSON {}", json).as_str());
                let message_json: serde_json::Value = match serde_json::from_str(&json) {
                    Ok(v) => v,
                    Err(_) => {
                        print_to_terminal(1, "apps_home: failed to parse ipc JSON, skipping");
                        continue;
                    },
                };

                print_to_terminal(1, "apps_home: parsed ipc JSON");

                if message_json["path"] == "/" && message_json["method"] == "GET" {
                    print_to_terminal(1, "apps_home: sending response");

                    send_response(
                        &Response {
                            ipc: Some(serde_json::json!({
                                "action": "response",
                                "status": 200,
                                "headers": {
                                    "Content-Type": "text/html",
                                },
                            }).to_string()),
                            metadata: None,
                        },
                        Some(&Payload {
                            mime: Some("text/html".to_string()),
                            bytes: APPS_HOME_PAGE.replace("${our}", &our.node).to_string().as_bytes().to_vec(),
                        }),
                    );
                } else if message_json["path"].is_string() {
                    send_response(
                        &Response {
                            ipc: Some(json!({
                                "action": "response",
                                "status": 404,
                                "headers": {
                                    "Content-Type": "text/html",
                                },
                            }).to_string()),
                            metadata: None,
                        },
                        Some(&Payload {
                            mime: Some("text/html".to_string()),
                            bytes: "Not Found"
                                .to_string()
                                .as_bytes()
                                .to_vec(),
                        }),
                    );
                } else {
                    if let Some(payload) = get_payload() {
                        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&payload.bytes) {
                            print_to_terminal(1, format!("JSON: {}", json).as_str());
                            if json["message"] == "ping" {
                                // WebSocket pushes are sent as requests
                                send_request(
                                    &Address {
                                        node: our.node.clone(),
                                        process: ProcessId::Name("encryptor".to_string()),
                                    },
                                    &Request {
                                        inherit: false,
                                        expects_response: false,
                                        ipc: Some(serde_json::json!({
                                            "EncryptAndForwardAction": {
                                                "channel_id": "apps_home",
                                                "forward_to": {
                                                    "node": our.node.clone(),
                                                    "process": {
                                                        "Name": "http_server"
                                                    }, // If the message passed in an ID then we could send to just that ID
                                                }, // node, process
                                                "json": Some(serde_json::json!({ // this is the JSON to forward
                                                    "WebSocketPush": {
                                                        "target": {
                                                            "node": our.node.clone(),
                                                            "id": "apps_home", // If the message passed in an ID then we could send to just that ID
                                                        }
                                                    }
                                                })),
                                            }

                                        }).to_string()),
                                        metadata: None,
                                    },
                                    None,
                                    Some(&Payload {
                                        mime: Some("application/json".to_string()),
                                        bytes: serde_json::json!({
                                            "pong": true
                                        }).to_string().as_bytes().to_vec(),
                                    }),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
