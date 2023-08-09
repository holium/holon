cargo_component_bindings::generate!();

use bindings::component::microkernel_process::types::WitMessageType;
use bindings::component::microkernel_process::types::WitPayload;
use bindings::component::microkernel_process::types::WitProcessNode;
use bindings::component::microkernel_process::types::WitProtomessageType;
use bindings::component::microkernel_process::types::WitRequestTypeWithTarget;
use std::collections::HashMap;
// use std::time::{SystemTime, UNIX_EPOCH};
// use jsonwebtoken::{decode, Validation, Algorithm, DecodingKey};
// use serde::Deserialize;
// use regex::Regex;

// #[derive(Debug, Deserialize)]
// struct JwtClaims {
//   sub: String,
//   exp: usize,
// }

// fn check_auth_token(our: String, secret: String, token: String) -> bool {
//   let validation = Validation::new(Algorithm::HS256);

//   let token_data = decode::<JwtClaims>(&token, &DecodingKey::from_secret(secret.as_ref()), &validation);

//   match token_data {
//       Ok(data) => {
//           let now = SystemTime::now();
//           let now_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
//           let now_since_epoch_as_usize = now_since_epoch.as_secs() as usize;
//           data.claims.sub == our && data.claims.exp > now_since_epoch_as_usize
//       },
//       Err(_) => false,
//   }
// }


struct Component;

impl bindings::MicrokernelProcess for Component {
    fn run_process(our: String, _dap: String) {
        bindings::print_to_terminal(1, "http_bindings: start");
        // TODO needs to be some kind of HttpPath => String
        let mut bindings: HashMap<String, String> = HashMap::new();

        loop {
            let (message, _) = bindings::await_next_message().unwrap();  //  TODO: handle error properly
            let Some(message_json_text) = message.content.payload.json else {
                panic!("foo")
            };
            let message_json: serde_json::Value = match serde_json::from_str(&message_json_text) {
                Ok(v) => v,
                Err(_) => {
                    bindings::print_to_terminal(1, "http_bindings: failed to parse message_json_text");
                    continue;
                },
            };

            match message.content.message_type {
                WitMessageType::Request(_) => {
                    let action = &message_json["action"];
                    // Safely unwrap the path as a string
                    let path = match message_json["path"].as_str() {
                        Some(s) => s,
                        None => "", // or any other default value
                    };
                    let app = match message_json["app"].as_str() {
                        Some(s) => s,
                        None => "", // or any other default value
                    };

                    if action == "bind-app" && path != "" && app != "" {
                        bindings.insert(path.to_string(), app.to_string());
                    } else if action == "request" {
                        bindings::print_to_terminal(1, "http_bindings: got request");

                        // // if the request path is "/", starts with "/~" or "/apps", then we need to check the uqbar-auth cookie
                        // let re = Regex::new(r"^/(~/.+|apps/.+|)$").unwrap();

                        // if re.is_match(message_json["path"].as_str().unwrap()) {
                        //     let cookie = message_json["headers"]["Cookie"].as_str().unwrap();
                        //     let cookie_parts: Vec<&str> = cookie.split("; ").collect();
                        //     let mut auth_token = None;
                        //     for cookie_part in cookie_parts {
                        //         let cookie_part_parts: Vec<&str> = cookie_part.split("=").collect();
                        //         if cookie_part_parts[0] == "uqbar-auth" {
                        //             auth_token = Some(cookie_part_parts[1].to_string());
                        //         }
                        //     }

                        //     // Check if the node has UqName registered with network keys
                        //     // If so, redirect to /login, otherwise redirect to /register
                        //     // Set the "location" header to the redirect URL and the status to 302

                        //     if auth_token.is_none() {
                        //         bindings::yield_results(vec![(
                        //             bindings::WitProtomessage {
                        //                 protomessage_type: WitProtomessageType::Response,
                        //                 payload: &WitPayload {
                        //                     json: Some(serde_json::json!({
                        //                         "id": message_json["id"],
                        //                         "status": 401,
                        //                         "headers": {"Content-Type": "text/plain"},

                        //                     }).to_string()),
                        //                     bytes: Some("Unauthorized".as_bytes().to_vec()),
                        //                 },
                        //             },
                        //             "",
                        //         )].as_slice());
                        //         continue;
                        //     }
                        //     let auth_token = auth_token.unwrap();
                        //     // Need to use the secret here
                        //     if !check_auth_token(our.clone(), _dap.clone(), auth_token) {
                        //         bindings::yield_results(vec![(
                        //             bindings::WitProtomessage {
                        //                 protomessage_type: WitProtomessageType::Response,
                        //                 payload: &WitPayload {
                        //                     json: Some(serde_json::json!({
                        //                         "id": message_json["id"],
                        //                         "status": 401,
                        //                         "headers": {"Content-Type": "text/plain"},

                        //                     }).to_string()),
                        //                     bytes: Some("Unauthorized".as_bytes().to_vec()),
                        //                 },
                        //             },
                        //             "",
                        //         )].as_slice());
                        //         continue;
                        //     }
                        // }

                        // let app = bindings.get(message_json["path"].as_str().unwrap()).unwrap();

                        let path_segments = path.trim_start_matches('/').split("/").collect::<Vec<&str>>();
                        let mut registered_path = path;
                        let mut url_params: HashMap<String, String> = HashMap::new();

                        for (key, _value) in &bindings {
                            let key_segments = key.trim_start_matches('/').split("/").collect::<Vec<&str>>();
                            if key_segments.len() != path_segments.len() && !key.contains("/.*") {
                                continue;
                            }

                            let mut paths_match = true;
                            for i in 0..key_segments.len() {
                                if key_segments[i] == ".*" {
                                    break;
                                } else if !(key_segments[i].starts_with(":") || key_segments[i] == path_segments[i]) {
                                    paths_match = false;
                                    break;
                                } else if key_segments[i].starts_with(":") {
                                    url_params.insert(key_segments[i][1..].to_string(), path_segments[i].to_string());
                                }
                            }

                            if paths_match {
                                registered_path = key;
                                break;
                            }
                        }

                        match bindings.get(registered_path) {
                            Some(app) => {
                                bindings::print_to_terminal(1, &("http_bindings: properly unwrapped path ".to_string() + registered_path));
                                bindings::yield_results(Ok(vec![(
                                    bindings::WitProtomessage {
                                        protomessage_type: WitProtomessageType::Request(
                                            WitRequestTypeWithTarget {
                                                is_expecting_response: false,
                                                target: WitProcessNode {
                                                    node: our.clone(),
                                                    process: app.into(),
                                                },
                                            }
                                        ),
                                        payload: WitPayload {
                                            json: Some(serde_json::json!({
                                                "path": registered_path,
                                                "raw_path": path,
                                                "method": message_json["method"],
                                                "headers": message_json["headers"],
                                                "query_params": message_json["query_params"],
                                                "url_params": url_params,
                                                "id": message_json["id"],
                                            }).to_string()),
                                            bytes: message.content.payload.bytes,
                                        },
                                    },
                                    "".into(),
                                )].as_slice()));
                            },
                            None => {
                                bindings::print_to_terminal(1, "http_bindings: no app found at this path");
                                bindings::yield_results(Ok(vec![(
                                    bindings::WitProtomessage {
                                        protomessage_type: WitProtomessageType::Response,
                                        payload: WitPayload {
                                            json: Some(serde_json::json!({
                                                "id": message_json["id"],
                                                "status": 404,
                                                "headers": {"Content-Type": "text/plain"},

                                            }).to_string()),
                                            bytes: Some("404 Not Found".as_bytes().to_vec()),
                                        },
                                    },
                                    "".into(),
                                )].as_slice()));
                            },
                        }
                    } else {
                        bindings::print_to_terminal(1,
                            format!(
                                "http_bindings: unexpected action: {:?}",
                                &message_json["action"],
                            ).as_str()
                        );
                    }
                },
                WitMessageType::Response => bindings::print_to_terminal(1, "http_bindings: got unexpected response"),
            }
        }
    }
}
