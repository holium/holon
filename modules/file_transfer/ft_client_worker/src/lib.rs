cargo_component_bindings::generate!();

use bindings::{MicrokernelProcess, print_to_terminal, receive};
use bindings::component::microkernel_process::types;

mod ft_types;
mod process_lib;

struct Component;

fn determine_if_resume(
    our: &types::ProcessAddress,
    resume_file_hash: Option<[u8; 32]>,
    chunk_size: u64,
) -> (Option<[u8; 32]>, u64) {
    let from_scratch = (None, 0);
    match determine_if_resume_inner(our, resume_file_hash, chunk_size) {
        Ok(r) => r,
        Err(e) => {
            print_to_terminal(
                1,
                &format!(
                    "ft_client_worker: failed to resume; starting from scratch. Error: {}",
                    e
                )
            );
            from_scratch
        },
    }
}

fn determine_if_resume_inner(
    our: &types::ProcessAddress,
    resume_file_hash: Option<[u8; 32]>,
    chunk_size: u64,
) -> anyhow::Result<(Option<[u8; 32]>, u64)> {
    let resume_file_hash = resume_file_hash.ok_or(anyhow::anyhow!(""))?;
    let length_response = process_lib::send_and_await_receive(
        our.node.clone(),
        types::ProcessIdentifier::Name("lfs".into()),
        Some(ft_types::FsAction::Length(resume_file_hash)),
        types::OutboundPayloadBytes::None,
    )??;
    let length_json =  process_lib::get_json(&length_response)?;
    let ft_types::FsResponse::Length(length) = process_lib::parse_message_json(Some(length_json))? else {
        return Err(anyhow::anyhow!(""));
    };
    if length % chunk_size != 0 {
        return Err(anyhow::anyhow!(""));
    }
    let piece_number = length / chunk_size;
    Ok((Some(resume_file_hash), piece_number))
}

fn handle_next_message(
    our: &types::ProcessAddress,
) -> anyhow::Result<()> {
    let (message, _context) = receive()?;

    match message {
        types::InboundMessage::Response(_) => Err(anyhow::anyhow!("unexpected Response")),
        types::InboundMessage::Request(types::InboundRequest {
            is_expecting_response: _,
            payload: types::InboundPayload {
                source: _,
                json,
                bytes: _,
            },
        }) => {
            match process_lib::parse_message_json(json)? {
                ft_types::FileTransferRequest::GetFile {
                    target_node,
                    file_hash,
                    chunk_size,
                    resume_file_hash,  //  TODO: resume if file exists
                } => {
                    //  (1): Start transfer with ft_server
                    //  (2): iteratively GetPiece and Append until have acquired whole file
                    //  (3): clean up

                    //  (1)
                    print_to_terminal(1, "a");
                    let start_response = process_lib::send_and_await_receive(
                        target_node.clone(),
                        types::ProcessIdentifier::Name("ft_server".into()),
                        Some(ft_types::FileTransferRequest::Start {
                            file_hash,
                            chunk_size,
                        }),
                        types::OutboundPayloadBytes::None,
                    )?;
                    print_to_terminal(1, "b");
                    let (metadata, source) = match start_response {
                        Err(e) => Err(anyhow::anyhow!("couldn't Start transfer from ft_server: {}", e)),
                        Ok(start_message) => {
                            let start_json = process_lib::get_json(&start_message)?;
                            match process_lib::parse_message_json(Some(start_json))? {
                                ft_types::FileTransferResponse::Start(metadata) => {
                                    Ok((metadata, process_lib::get_source(&start_message)))
                                },
                                _ => Err(anyhow::anyhow!("unexpected Response resulting from Start transfer from ft_server")),
                            }
                        },
                    }?;
                    print_to_terminal(1, "c");
                    assert_eq!(target_node, source.node);
                    let (current_file_hash, next_piece_number) = determine_if_resume(
                        &our,
                        resume_file_hash,
                        chunk_size,
                    );
                    let mut state = ft_types::ClientWorkerState {
                        metadata,
                        current_file_hash,
                        next_piece_number,
                    };
                    print_to_terminal(1, "d");

                    //  (2)
                    while state.metadata.number_pieces > state.next_piece_number {
                        //  TODO: circumvent bytes?
                        let get_piece_response = process_lib::send_and_await_receive(
                            source.node.clone(),
                            source.identifier.clone(),
                            Some(&ft_types::FileTransferRequest::GetPiece {
                                piece_number: state.next_piece_number,
                            }),
                            types::OutboundPayloadBytes::None,
                        )?;
                        let bytes = process_lib::get_bytes(
                            match get_piece_response{
                                Err(e) => Err(anyhow::anyhow!("unexpected Response resulting from Start transfer from ft_server: {:?}", e)),
                                Ok(gpr) => Ok(gpr),
                            }?
                        )?;

                        let append_response = process_lib::send_and_await_receive(
                            our.node.clone(),
                            types::ProcessIdentifier::Name("lfs".into()),
                            Some(&ft_types::FsAction::Append(state.current_file_hash)), // lfs interface will reflect this
                            types::OutboundPayloadBytes::Some(bytes),
                        )?;
                        let file_hash = match append_response {
                            Err(e) => Err(anyhow::anyhow!("couldn't Append file piece: {}", e)),
                            Ok(append_message) => {
                                let append_json = process_lib::get_json(&append_message)?;
                                match process_lib::parse_message_json(Some(append_json))? {
                                    ft_types::FsResponse::Append(file_hash) => Ok(file_hash),
                                    _ => Err(anyhow::anyhow!("unexpected Response resulting from lfs Append")),
                                }
                            },
                        }?;
                        state.current_file_hash = Some(file_hash.clone());

                        let _ = process_lib::send_and_await_receive(
                            our.node.clone(),
                            types::ProcessIdentifier::Name("ft_client".into()),
                            Some(&ft_types::FileTransferRequest::UpdateClientState {
                                current_file_hash: file_hash,
                            }),
                            types::OutboundPayloadBytes::None,
                        )?;

                        state.next_piece_number += 1;
                    }

                    //  (3)
                    assert_eq!(Some(state.metadata.key.file_hash), state.current_file_hash);
                    print_to_terminal(0, &format!(
                        "file_transfer: successfully downloaded {:?} from {}",
                        state.metadata.key.file_hash,
                        state.metadata.key.server,
                    ));
                    process_lib::send_one_request(
                        false,
                        &state.metadata.key.server,
                        source.identifier.clone(),
                        Some(ft_types::FileTransferRequest::Done),
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
        print_to_terminal(1, "ft_client_worker: begin");

        match handle_next_message(&our) {
            Ok(_) => { return; },
            Err(e) => {
                print_to_terminal(0, &format!("ft_client_worker: error: {:?}", e));
                panic!();
            },
        };
    }
}
