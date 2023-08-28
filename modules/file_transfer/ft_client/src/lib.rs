cargo_component_bindings::generate!();

use std::collections::HashMap;

use bindings::{MicrokernelProcess, print_to_terminal, receive};
use bindings::component::microkernel_process::types;

mod ft_types;
mod process_lib;

struct Component;

fn de_wit_process_reference(wit: &types::ProcessReference) -> ft_types::ProcessReference {
    ft_types::ProcessReference {
        node: wit.node.clone(),
        identifier: de_wit_process_identifier(&wit.identifier),
    }
}
fn de_wit_process_identifier(wit: &types::ProcessIdentifier) -> ft_types::ProcessIdentifier {
    match wit {
        types::ProcessIdentifier::Id(id) => ft_types::ProcessIdentifier::Id(id.clone()),
        types::ProcessIdentifier::Name(name) => ft_types::ProcessIdentifier::Name(name.clone()),
    }
}

fn handle_next_message(
    our: &types::ProcessAddress,
    state: &mut ft_types::ClientState,
) -> anyhow::Result<()> {
    let (message, _context) = receive()?;

    match message {
        types::InboundMessage::Response(_) => Err(anyhow::anyhow!("unexpected Response")),
        types::InboundMessage::Request(types::InboundRequest {
            is_expecting_response: _,
            payload: types::InboundPayload {
                source,
                json,
                bytes: _,
            },
        }) => {
            match process_lib::parse_message_json(json)? {
                //  TODO: maintain & persist state about ongoing transfers
                //        resume rather than starting from scratch when appropriate
                ft_types::FileTransferRequest::GetFile {
                    target_node,
                    file_hash,
                    chunk_size,
                    resume_file_hash,
                } => {
                    //  (1) spin up ft_client_worker to handle upload
                    //  (2) add to state
                    //  (3) send GetFile to client_worker to begin download

                    //  (1)
                    let response = process_lib::send_and_await_receive(
                        our.node.clone(),
                        types::ProcessIdentifier::Name("process_manager".into()),
                        Some(ft_types::ProcessManagerCommand::Start {
                            name: None,
                            wasm_bytes_uri: "fs://sequentialize/file_transfer/ft_client_worker.wasm".into(),  //  TODO; should this be persisted when it becomes a file hash?
                            send_on_panic: ft_types::SendOnPanic::None,
                            //  TODO: inform client and/or server_worker?
                            // send_on_panic: SendOnPanic::Requests(vec![
                            //     RequestOnPanic {
                            //         target: ProcessReference {
                            //         },
                            //         json: ,
                            //         bytes: TransitPayloadBytes::None,
                            //     },
                            // ]),
                        }),
                        types::OutboundPayloadBytes::None,
                    )?;
                    let id = match response {
                        Err(e) => Err(anyhow::anyhow!("couldn't Start ft_client_worker: {}", e)),
                        Ok(response_message) => {
                            let response_json = process_lib::get_json(&response_message)?;
                            match process_lib::parse_message_json(Some(response_json))? {
                                ft_types::ProcessManagerResponse::Start { id, name: _ } => Ok(id),
                                _ => Err(anyhow::anyhow!("unexpected Response resulting from Start ft_client_worker")),
                            }
                        },
                    }?;

                    //  (2)
                    state.insert(
                        ft_types::ProcessReference {
                            node: our.node.clone(),
                            identifier: ft_types::ProcessIdentifier::Id(id.clone()),
                        },
                        ft_types::ClientStateValue {
                            target_node: target_node.clone(),
                            file_hash: file_hash.clone(),
                            chunk_size: chunk_size.clone(),
                            current_file_hash: None,
                        },
                    );

                    //  (3)
                    process_lib::send_one_request(
                        false,
                        &our.node,
                        types::ProcessIdentifier::Id(id),
                        Some(ft_types::FileTransferRequest::GetFile {
                            target_node,
                            file_hash,
                            chunk_size,
                            resume_file_hash,
                        }),
                        types::OutboundPayloadBytes::None,
                        None::<ft_types::FileTransferContext>,
                    )?;
                    Ok(())
                },
                ft_types::FileTransferRequest::UpdateClientState { current_file_hash } => {
                    let s  = state.get_mut(&de_wit_process_reference(&source));
                    s.current_file_hash = Some(current_file_hash);
                    process_lib::persist_state(&our.node, state)?;

                    process_lib::send_response(
                        Some(ft_types::FileTransferResponse::UpdateClientState),
                        types::OutboundPayloadBytes::None,
                        None::<ft_types::FileTransferContext>,
                    )?;

                    Ok(())
                },
                _ => Err(anyhow::anyhow!("unexpected Request")),
            }
        }
    }
}

impl MicrokernelProcess for Component {
    fn run_process(our: types::ProcessAddress) {
        print_to_terminal(1, "ft_client: begin");

        let mut state: ft_types::ClientState = HashMap::new();

        loop {
            match handle_next_message(
                &our,
                &mut state,
            ) {
                Ok(_) => {},
                Err(e) => {
                    //  TODO: should bail?
                    print_to_terminal(0, &format!("ft_client: error: {:?}", e));
                },
            };
        }
    }
}
