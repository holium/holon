use std::str::FromStr;

use nectar_process_lib::{
    await_message, our_capabilities, println, spawn, vfs, Address, Message, OnExit, ProcessId, Request, Response,
};

mod tester_types;
use tester_types as tt;

wit_bindgen::generate!({
    path: "../../../wit",
    world: "process",
    exports: {
        world: Component,
    },
});

fn make_vfs_address(our: &Address) -> anyhow::Result<Address> {
    Ok(Address::new(
        our.node.clone(),
        ProcessId::from_str("vfs:sys:nectar")?,
    ))
}

fn handle_message(our: &Address) -> anyhow::Result<()> {
    let message = await_message().unwrap();

    match message {
        Message::Response { .. } => {
            return Err(tt::TesterError::UnexpectedResponse.into());
        }
        Message::Request { ref body, .. } => {
            match serde_json::from_slice(body)? {
                tt::TesterRequest::Run { test_timeout, .. } => {
                    println!("test_runner: got Run");

                    let response = Request::new()
                        .target(make_vfs_address(&our)?)
                        .body(serde_json::to_vec(&vfs::VfsRequest {
                            path: "/tester:nectar/tests".into(),
                            action: vfs::VfsAction::ReadDir,
                        })?)
                        .send_and_await_response(test_timeout)?
                        .unwrap();

                    let Message::Response { body: vfs_body, .. } = response else {
                        panic!("")
                    };
                    let vfs::VfsResponse::ReadDir(mut children) = serde_json::from_slice(&vfs_body)?
                    else {
                        println!(
                            "{:?}",
                            serde_json::from_slice::<serde_json::Value>(&vfs_body)?
                        );
                        panic!("")
                    };

                    let caps_file_path = "tester:nectar/tests/grant_capabilities.json";
                    let caps_index = children.iter().position(|i| *i.path == *caps_file_path);
                    let caps_by_child: std::collections::HashMap<String, Vec<String>> = match caps_index {
                        None => std::collections::HashMap::new(),
                        Some(caps_index) => {
                            children.remove(caps_index);
                            let file = vfs::file::open_file(caps_file_path, false)?;
                            let file_contents = file.read()?;
                            serde_json::from_slice(&file_contents)?
                        }
                    };

                    println!("test_runner: running {:?}...", children);

                    for child in &children {
                        let grant_caps = child.path
                            .split("/")
                            .last()
                            .and_then(|child_file_name| child_file_name.strip_suffix(".wasm"))
                            .and_then(|child_file_name| {
                                caps_by_child
                                    .get(child_file_name)
                                    .and_then(|caps| Some(caps.iter().map(|cap| ProcessId::from_str(cap).unwrap()).collect()))
                            })
                            .unwrap_or(vec![]);
                        let child_process_id = match spawn(
                            None,
                            &child.path,
                            OnExit::None, //  TODO: notify us
                            our_capabilities(),
                            grant_caps,
                            false, // not public
                        ) {
                            Ok(child_process_id) => child_process_id,
                            Err(e) => {
                                println!("couldn't spawn {}: {}", child.path, e);
                                panic!("couldn't spawn"); //  TODO
                            }
                        };

                        let response = Request::new()
                            .target(Address {
                                node: our.node.clone(),
                                process: child_process_id,
                            })
                            .body(body.clone())
                            .send_and_await_response(test_timeout)?
                            .unwrap();

                        let Message::Response { body, .. } = response else {
                            panic!("")
                        };
                        match serde_json::from_slice(&body)? {
                            tt::TesterResponse::Pass => {}
                            tt::TesterResponse::GetFullMessage(_) => {}
                            tt::TesterResponse::Fail {
                                test,
                                file,
                                line,
                                column,
                            } => {
                                fail!(test, file, line, column);
                            }
                        }
                    }

                    println!("test_runner: done running {:?}", children);

                    Response::new()
                        .body(serde_json::to_vec(&tt::TesterResponse::Pass).unwrap())
                        .send()
                        .unwrap();
                }
                tt::TesterRequest::KernelMessage(_) | tt::TesterRequest::GetFullMessage(_) => {
                    unimplemented!()
                }
            }
            Ok(())
        }
    }
}

struct Component;
impl Guest for Component {
    fn init(our: String) {
        println!("{:?}@test_runner: begin", our);

        let our: Address = our.parse().unwrap();

        loop {
            match handle_message(&our) {
                Ok(()) => {}
                Err(e) => {
                    println!("test_runner: error: {:?}", e);
                    fail!("test_runner");
                }
            };
        }
    }
}
