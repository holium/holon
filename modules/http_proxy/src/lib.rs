cargo_component_bindings::generate!();

use std::collections::HashMap;
use serde_json::json;
use serde::{Serialize, Deserialize};

use bindings::component::microkernel_process::types;

#[derive(Debug, Serialize, Deserialize)]
pub enum FileSystemAction {
    Read,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileSystemRequest {
    pub uri_string: String,
    pub action: FileSystemAction,
}

mod process_lib;

const PROXY_HOME_PAGE: &str = include_str!("http-proxy.html");

struct Component;

impl bindings::MicrokernelProcess for Component {
    fn run_process(our_name: String, process_name: String) {
        bindings::print_to_terminal(1, "http-proxy: start");
        bindings::send_requests(Ok((
            vec![
                types::WitProtorequest {
                    is_expecting_response: false,
                    target: types::WitProcessNode {
                        node: our_name.clone(),
                        process: "http_bindings".into(),
                    },
                    payload: types::WitPayload {
                        json: Some(serde_json::json!({
                            "action": "bind-app",
                            "path": "/http-proxy",
                            "authenticated": true,
                            "app": process_name
                        }).to_string()),
                        bytes: types::WitPayloadBytes {
                            circumvent: types::WitCircumvent::False,
                            content: None,
                        },
                    }
                },
                types::WitProtorequest {
                    is_expecting_response: false,
                    target: types::WitProcessNode {
                        node: our_name.clone(),
                        process: "http_bindings".into(),
                    },
                    payload: types::WitPayload {
                        json: Some(serde_json::json!({
                            "action": "bind-app",
                            "path": "/http-proxy/static/.*",
                            "authenticated": true,
                            "app": process_name
                        }).to_string()),
                        bytes: types::WitPayloadBytes {
                            circumvent: types::WitCircumvent::False,
                            content: None,
                        },
                    }
                },
                types::WitProtorequest {
                    is_expecting_response: false,
                    target: types::WitProcessNode {
                        node: our_name.clone(),
                        process: "http_bindings".into(),
                    },
                    payload: types::WitPayload {
                        json: Some(serde_json::json!({
                            "action": "bind-app",
                            "path": "/http-proxy/list",
                            "app": process_name
                        }).to_string()),
                        bytes: types::WitPayloadBytes {
                            circumvent: types::WitCircumvent::False,
                            content: None,
                        },
                    }
                },
                types::WitProtorequest {
                    is_expecting_response: false,
                    target: types::WitProcessNode {
                        node: our_name.clone(),
                        process: "http_bindings".into(),
                    },
                    payload: types::WitPayload {
                        json: Some(serde_json::json!({
                            "action": "bind-app",
                            "path": "/http-proxy/register",
                            "app": process_name
                        }).to_string()),
                        bytes: types::WitPayloadBytes {
                            circumvent: types::WitCircumvent::False,
                            content: None,
                        },
                    }
                },
                types::WitProtorequest {
                    is_expecting_response: false,
                    target: types::WitProcessNode {
                        node: our_name.clone(),
                        process: "http_bindings".into(),
                    },
                    payload: types::WitPayload {
                        json: Some(serde_json::json!({
                            "action": "bind-app",
                            "path": "/http-proxy/serve/:username/.*",
                            "app": process_name
                        }).to_string()),
                        bytes: types::WitPayloadBytes {
                            circumvent: types::WitCircumvent::False,
                            content: None,
                        },
                    }
                },
            ].as_slice(),
            "".into(),
        )));

        let mut registrations: HashMap<String, String> = HashMap::new();

        loop {
            let (message, _) = bindings::await_next_message().unwrap();  //  TODO: handle error properly
            let Some(message_from_loop_string) = message.content.payload.json else {
                panic!("foo")
            };
            let message_from_loop: serde_json::Value = serde_json::from_str(&message_from_loop_string).unwrap();
            bindings::print_to_terminal(1, format!("http-proxy: got request: {}", message_from_loop).as_str());

            if message_from_loop["path"] == "/http-proxy" && message_from_loop["method"] == "GET" {
                bindings::send_response(Ok((
                    &types::WitPayload {
                        json: Some(serde_json::json!({
                            "action": "response",
                            "status": 200,
                            "headers": {
                                "Content-Type": "text/html",
                            },
                        }).to_string()),
                        bytes: types::WitPayloadBytes {
                            circumvent: types::WitCircumvent::False,
                            content: Some(PROXY_HOME_PAGE.replace("${our}", &our_name).as_bytes().to_vec()),
                        },
                    },
                    "",
                )));
            } else if message_from_loop["path"] == "/http-proxy/list" && message_from_loop["method"] == "GET" {
                bindings::send_response(Ok((
                    &types::WitPayload {
                        json: Some(serde_json::json!({
                            "action": "response",
                            "status": 200,
                            "headers": {
                                "Content-Type": "application/json",
                            },
                        }).to_string()),
                        bytes: types::WitPayloadBytes {
                            circumvent: types::WitCircumvent::False,
                            content: Some(serde_json::json!({
                                "registrations": registrations
                            })
                                .to_string()
                                .as_bytes()
                                .to_vec()),
                        },
                    },
                    "",
                )));
            } else if message_from_loop["path"] == "/http-proxy/register" && message_from_loop["method"] == "POST" {
                let mut status = 204;
                let body_bytes = message.content.payload.bytes.content.unwrap_or(vec![]);
                let body_json_string = match String::from_utf8(body_bytes) {
                    Ok(s) => s,
                    Err(_) => String::new()
                };
                let body: serde_json::Value = serde_json::from_str(&body_json_string).unwrap();
                let username = body["username"].as_str().unwrap_or("");

                bindings::print_to_terminal(1, format!("Register proxy for: {}", username).as_str());

                if !username.is_empty() {
                    registrations.insert(username.to_string(), "foo".to_string());
                } else {
                    status = 400;
                }

                bindings::send_response(Ok((
                    &types::WitPayload {
                        json: Some(serde_json::json!({
                            "action": "response",
                            "status": status,
                            "headers": {
                                "Content-Type": "text/html",
                            },
                        }).to_string()),
                        bytes: types::WitPayloadBytes {
                            circumvent: types::WitCircumvent::False,
                            content: Some((if status == 400 { "Bad Request" } else { "Success" }).to_string().as_bytes().to_vec()),
                        },
                    },
                    "",
                )));
            } else if message_from_loop["path"] == "/http-proxy/register" && message_from_loop["method"] == "DELETE" {
                bindings::print_to_terminal(1, "HERE IN /http-proxy/register to delete something");
                let username = message_from_loop["query_params"]["username"].as_str().unwrap_or("");

                let mut status = 204;

                if !username.is_empty() {
                    registrations.remove(username);
                } else {
                    status = 400;
                }

                // TODO when we have an actual webpage, uncomment this as a response
                bindings::send_response(Ok((
                    &types::WitPayload {
                        json: Some(serde_json::json!({
                            "action": "response",
                            "status": status,
                            "headers": {
                                "Content-Type": "text/html",
                            },
                        }).to_string()),
                        bytes: types::WitPayloadBytes {
                            circumvent: types::WitCircumvent::False,
                            content: Some((if status == 400 { "Bad Request" } else { "Success" }).to_string().as_bytes().to_vec()),
                        },
                    },
                    "",
                )));
            } else if message_from_loop["path"] == "/http-proxy/serve/:username/.*" {
                let username = message_from_loop["url_params"]["username"].as_str().unwrap_or("");
                let raw_path = message_from_loop["raw_path"].as_str().unwrap_or("");
                bindings::print_to_terminal(1, format!("proxy for user: {}", username).as_str());

                if username.is_empty() || raw_path.is_empty() {
                    bindings::send_response(Ok((
                        &types::WitPayload {
                            json: Some(serde_json::json!({
                                "action": "response",
                                "status": 404,
                                "headers": {
                                    "Content-Type": "text/html",
                                },
                            }).to_string()),
                            bytes: types::WitPayloadBytes {
                                circumvent: types::WitCircumvent::False,
                                content: Some("Not Found".to_string().as_bytes().to_vec()),
                            },
                        },
                        "",
                    )));
                } else if !registrations.contains_key(username) {
                    bindings::send_response(Ok((
                        &types::WitPayload {
                            json: Some(serde_json::json!({
                                "action": "response",
                                "status": 403,
                                "headers": {
                                    "Content-Type": "text/html",
                                },
                            }).to_string()),
                            bytes: types::WitPayloadBytes {
                                circumvent: types::WitCircumvent::False,
                                content: Some("Not Authorized".to_string().as_bytes().to_vec()),
                            },
                        },
                        "",
                    )));
                } else {
                    let path_parts: Vec<&str> = raw_path.split('/').collect();
                    let mut proxied_path = "/".to_string();

                    if let Some(pos) = path_parts.iter().position(|&x| x == "serve") {
                        proxied_path = format!("/{}", path_parts[pos+2..].join("/"));
                        bindings::print_to_terminal(1, format!("Path to proxy: {}", proxied_path).as_str());
                    }

                    let res = process_lib::send_request_and_await_response(
                        username.into(),
                        "http_bindings".into(),
                        Some(json!({
                            "action": "request",
                            "method": message_from_loop["method"],
                            "path": proxied_path,
                            "headers": message_from_loop["headers"],
                            "proxy_path": raw_path,
                            "query_params": message_from_loop["query_params"],
                        })),
                        message.content.payload.bytes,
                    ).unwrap(); // TODO unwrap
                    bindings::print_to_terminal(1, "FINISHED YIELD AND AWAIT");
                    match res.content.payload.json {
                        Some(ref json) => {
                            if json.contains("Offline") {
                                bindings::send_response(Ok((
                                    &types::WitPayload {
                                        json: Some(serde_json::json!({
                                            "status": 404,
                                            "headers": {
                                                "Content-Type": "text/html"
                                            },
                                        }).to_string()),
                                        bytes: types::WitPayloadBytes {
                                            circumvent: types::WitCircumvent::False,
                                            content: Some("Node is offline".as_bytes().to_vec()),
                                        },
                                    },
                                    "".into(),
                                )))
                            } else {
                                bindings::send_response(Ok((
                                    &types::WitPayload {
                                        json: res.content.payload.json,
                                        bytes: res.content.payload.bytes,
                                    },
                                    "".into(),
                                )))
                            }
                        },
                        None => bindings::send_response(Ok((
                            &types::WitPayload {
                                json: Some(serde_json::json!({
                                    "status": 404,
                                    "headers": {
                                        "Content-Type": "text/html"
                                    },
                                }).to_string()),
                                bytes: types::WitPayloadBytes {
                                    circumvent: types::WitCircumvent::False,
                                    content: Some("Not Found".as_bytes().to_vec()),
                                },
                            },
                            "".into(),
                        ))),
                    };
                }
            } else {
                bindings::send_response(Ok((
                    &types::WitPayload {
                        json: Some(serde_json::json!({
                            "action": "response",
                            "status": 404,
                            "headers": {
                                "Content-Type": "text/html",
                            },
                        }).to_string()),
                        bytes: types::WitPayloadBytes {
                            circumvent: types::WitCircumvent::False,
                            content: Some("Not Found".as_bytes().to_vec()),
                        },
                    },
                    "".into(),
                )));
            }
        }
    }
}
