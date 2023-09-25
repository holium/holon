/// log structured filesystem
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use tokio::fs;
use tokio::time::{interval, Duration};

use crate::filesystem::manifest::{FileIdentifier, Manifest};
use crate::types::*;
mod manifest;

const CHUNK_SIZE: usize = 262144; // 256kb

pub async fn bootstrap(
    our_name: String,
    home_directory_path: String,
) -> Result<(ProcessMap, Manifest), FileSystemError> {
    // fs bootstrapping, create home_directory, fs directory inside it, manifest + log if none.
    if let Err(e) = create_dir_if_dne(&home_directory_path).await {
        panic!("{}", e);
    }
    let fs_directory_path_str = format!("{}/fs", &home_directory_path);

    if let Err(e) = create_dir_if_dne(&fs_directory_path_str).await {
        panic!("{}", e);
    }
    let fs_directory_path: std::path::PathBuf =
        fs::canonicalize(fs_directory_path_str).await.unwrap();

    //  open and load manifest+log

    let manifest_path = fs_directory_path.join("manifest.bin");

    let manifest_file = fs::OpenOptions::new()
        .append(true)
        .read(true)
        .create(true)
        .open(&manifest_path)
        .await
        .expect("fs: failed to open manifest file");

    let wal_path = fs_directory_path.join("wal.bin");

    let wal_file = fs::OpenOptions::new()
        .append(true)
        .read(true)
        .create(true)
        .open(&wal_path)
        .await
        .expect("fs: failed to open WAL file");

    //  in memory details about files.
    let manifest = Manifest::load(manifest_file, wal_file, &fs_directory_path)
        .await
        .expect("manifest load failed!");
    // mimic the FsAction::GetState case and get current state of ProcessId::name("kernel")
    // serialize it to a ProcessHandles from process id to JoinHandle

    let kernel_process_id = FileIdentifier::Process(ProcessId::Name("kernel".into()));
    let mut process_map: ProcessMap = HashMap::new();

    // get current processes' wasm_bytes handles. GetState(kernel)
    match manifest.read(&kernel_process_id, None, None).await {
        Err(_) => {
            //  first time!
        }
        Ok(bytes) => {
            process_map = bincode::deserialize(&bytes).expect("state map deserialization error!");
        }
    }

    // NOTE OnPanic
    // wasm bytes initial source of truth is the compiled .wasm file on-disk, but onPanic needs to come from somewhere to.
    // for apps in /modules, special cases can be defined here.

    // we also add special-case capabilities spawned "out of thin air" here, for distro processes.
    // at the moment, all bootstrapped processes are given the capability to message all others.
    // this can be easily changed in the future.
    // they are also given access to all runtime modules by name
    let names_and_bytes = get_processes_from_directories().await;
    const RUNTIME_MODULES: [&str; 8] = [
        "filesystem",
        "http_server",
        "http_client",
        "encryptor",
        "lfs",
        "net",
        "vfs",
        "kernel",
    ];

    let mut special_capabilities: HashSet<Capability> = HashSet::new();
    for (process_name, _) in &names_and_bytes {
        special_capabilities.insert(Capability {
            issuer: Address {
                node: our_name.clone(),
                process: ProcessId::Name(process_name.into()),
            },
            params: format!(
                "{{\"messaging\": \"{}\"}}",
                serde_json::to_string(&ProcessId::Name(process_name.into())).unwrap()
            ),
        });
    }
    for runtime_module in RUNTIME_MODULES {
        special_capabilities.insert(Capability {
            issuer: Address {
                node: our_name.clone(),
                process: ProcessId::Name(runtime_module.into()),
            },
            params: format!(
                "{{\"messaging\": \"{}\"}}",
                serde_json::to_string(&ProcessId::Name(runtime_module.into())).unwrap()
            ),
        });
    }
    // give all distro processes the ability to send messages across the network
    special_capabilities.insert(Capability {
        issuer: Address {
            node: our_name.clone(),
            process: ProcessId::Name("kernel".into()),
        },
        params: "\"network\"".into(),
    });

    // for a module in /modules, put its bytes into filesystem, add to process_map
    for (process_name, wasm_bytes) in names_and_bytes {
        let hash: [u8; 32] = hash_bytes(&wasm_bytes);

        if let Some(id) = manifest.get_uuid_by_hash(&hash).await {
            let entry =
                process_map
                    .entry(ProcessId::Name(process_name))
                    .or_insert(PersistedProcess {
                        wasm_bytes_handle: id,
                        on_panic: OnPanic::Restart,
                        capabilities: HashSet::new(),
                    });
            entry.capabilities.extend(special_capabilities.clone());
        } else {
            //  FsAction::Write
            let file = FileIdentifier::new_uuid();

            let _ = manifest.write(&file, &wasm_bytes).await;
            let id = file.to_uuid().unwrap();

            let entry =
                process_map
                    .entry(ProcessId::Name(process_name))
                    .or_insert(PersistedProcess {
                        wasm_bytes_handle: id,
                        on_panic: OnPanic::Restart,
                        capabilities: HashSet::new(),
                    });
            entry.capabilities.extend(special_capabilities.clone());
        }
    }

    // finally, save runtime modules in state map as well, somewhat fakely
    for runtime_module in RUNTIME_MODULES {
        let entry = process_map
            .entry(ProcessId::Name(runtime_module.into()))
            .or_insert(PersistedProcess {
                wasm_bytes_handle: 0,
                on_panic: OnPanic::Restart,
                capabilities: HashSet::new(),
            });
        entry.capabilities.extend(special_capabilities.clone());
    }

    // save kernel process state. FsAction::SetState(kernel)
    let serialized_process_map =
        bincode::serialize(&process_map).expect("state map serialization error!");
    let process_map_hash: [u8; 32] = hash_bytes(&serialized_process_map);

    if manifest.get_by_hash(&process_map_hash).await.is_none() {
        let _ = manifest
            .write(&kernel_process_id, &serialized_process_map)
            .await;
    }

    Ok((process_map, manifest))
}

async fn get_processes_from_directories() -> Vec<(String, Vec<u8>)> {
    let mut processes = Vec::new();

    // Get the path to the /modules directory
    let modules_path = std::path::Path::new("modules");

    // Read the /modules directory
    if let Ok(mut entries) = fs::read_dir(modules_path).await {
        // Loop through the entries in the directory
        while let Ok(Some(entry)) = entries.next_entry().await {
            // If the entry is a directory, add its name to the list of processes
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        // Get the path to the wasm file for the process
                        let wasm_path = format!(
                            "modules/{}/target/wasm32-unknown-unknown/release/{}.wasm",
                            name, name
                        );
                        // Read the wasm file
                        if let Ok(wasm_bytes) = fs::read(wasm_path).await {
                            // Add the process name and wasm bytes to the list of processes
                            processes.push((name.to_string(), wasm_bytes));
                        }
                    }
                }
            }
        }
    }

    processes
}

pub async fn fs_sender(
    our_name: String,
    manifest: Manifest,
    send_to_loop: MessageSender,
    send_to_terminal: PrintSender,
    mut recv_in_fs: MessageReceiver,
) -> Result<()> {
    //  interval for deleting/(flushing), don't want to run immediately upon bootup.
    let mut interval = interval(Duration::from_secs(60));
    let mut first_open = true;

    //  into main loop
    loop {
        tokio::select! {
            Some(kernel_message) = recv_in_fs.recv() => {
                if our_name != kernel_message.source.node {
                    println!(
                        "fs: request must come from our_name={}, got: {}",
                        our_name, &kernel_message,
                    );
                    continue;
                }

            //  internal structures have Arc::clone setup.
            let manifest_clone = manifest.clone();

            let our_name = our_name.clone();
            let source = kernel_message.source.clone();
            let id = kernel_message.id;
            let send_to_loop = send_to_loop.clone();
            let send_to_terminal = send_to_terminal.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_request(
                    our_name.clone(),
                    kernel_message,
                    manifest_clone,
                    send_to_loop.clone(),
                    send_to_terminal,
                )
                .await
                {
                    send_to_loop
                        .send(make_error_message(our_name.into(), id, source, e))
                        .await
                        .unwrap();
                }
            });
            }
            _ = interval.tick() => {
                if !first_open {
                    let manifest_clone = manifest.clone();

                    tokio::spawn(async move {
                        // the configuration of how often, and which one happens,
                        // should be decided upon boot.
                        let _ = manifest_clone.flush().await;
                        let _ = manifest_clone.cleanup().await;
                    });
                }
                first_open = false;
            }
        }
    }
}

async fn handle_request(
    our_name: String,
    kernel_message: KernelMessage,
    manifest: Manifest,
    send_to_loop: MessageSender,
    _send_to_terminal: PrintSender,
) -> Result<(), FileSystemError> {
    let KernelMessage {
        id,
        source,
        rsvp,
        message,
        payload,
        ..
    } = kernel_message;
    let Message::Request(Request {
        expects_response,
        ipc: Some(json_string),
        metadata, // for kernel
        ..
    }) = message
    else {
        return Err(FileSystemError::BadJson {
            json: "".into(),
            error: "not a Request with payload".into(),
        });
    };

    let action: FsAction = match serde_json::from_str(&json_string) {
        Ok(r) => r,
        Err(e) => {
            return Err(FileSystemError::BadJson {
                json: json_string.into(),
                error: format!("parse failed: {:?}", e),
            })
        }
    };

    //  println!("got action! {:?}", action);

    let (ipc, bytes) = match action {
        FsAction::Write => {
            let Some(ref payload) = payload else {
                return Err(FileSystemError::BadBytes {
                    action: "Write".into(),
                });
            };

            let file_uuid = FileIdentifier::new_uuid();
            manifest.write(&file_uuid, &payload.bytes).await?;

            (FsResponse::Write(file_uuid.to_uuid().unwrap()), None)
        }
        FsAction::WriteOffset((file_uuid, offset)) => {
            let Some(ref payload) = payload else {
                return Err(FileSystemError::BadBytes {
                    action: "Write".into(),
                });
            };

            let file_uuid = FileIdentifier::UUID(file_uuid);

            manifest
                .write_at(&file_uuid, offset, &payload.bytes)
                .await?;

            (FsResponse::Write(file_uuid.to_uuid().unwrap()), None)
        }
        FsAction::Read(file_uuid) => {
            let file = FileIdentifier::UUID(file_uuid);

            match manifest.read(&file, None, None).await {
                Err(e) => return Err(e),
                Ok(bytes) => (FsResponse::Read(file_uuid), Some(bytes)),
            }
        }
        FsAction::ReadChunk(req) => {
            let file = FileIdentifier::UUID(req.file);

            match manifest
                .read(&file, Some(req.start), Some(req.length))
                .await
            {
                Err(e) => return Err(e),
                Ok(bytes) => (FsResponse::Read(req.file), Some(bytes)),
            }
        }
        FsAction::Replace(old_file_uuid) => {
            let Some(ref payload) = payload else {
                return Err(FileSystemError::BadBytes {
                    action: "Write".into(),
                });
            };

            let file = FileIdentifier::UUID(old_file_uuid);
            manifest.write(&file, &payload.bytes).await?;

            (FsResponse::Write(old_file_uuid), None)
        }
        FsAction::Delete(del) => {
            let file = FileIdentifier::UUID(del);
            manifest.delete(&file).await?;

            (FsResponse::Delete(del), None)
        }
        FsAction::Append(maybe_file_uuid) => {
            let Some(ref payload) = payload else {
                return Err(FileSystemError::BadBytes {
                    action: "Append".into(),
                });
            };

            let file_uuid = match maybe_file_uuid {
                Some(uuid) => FileIdentifier::UUID(uuid),
                None => FileIdentifier::new_uuid(),
            };

            manifest.append(&file_uuid, &payload.bytes).await?;
            // note expecting file_uuid here, if we want process state to access append, we would change this.
            (FsResponse::Append(file_uuid.to_uuid().unwrap()), None)
        }
        FsAction::Length(file_uuid) => {
            let file = FileIdentifier::UUID(file_uuid);
            let length = manifest.get_length(&file).await;
            match length {
                Some(len) => (FsResponse::Length(len), None),
                None => {
                    return Err(FileSystemError::LFSError {
                        error: "file not found".into(),
                    })
                }
            }
        }
        FsAction::SetLength((file_uuid, length)) => {
            let file = FileIdentifier::UUID(file_uuid);
            manifest.set_length(&file, length).await?;

            // doublecheck if this is the type of return statement we want.
            (FsResponse::Length(length), None)
        }
        //  process state handlers
        FsAction::SetState => {
            let Some(ref payload) = payload else {
                return Err(FileSystemError::BadBytes {
                    action: "Write".into(),
                });
            };

            let file = FileIdentifier::Process(source.process.clone());
            let _ = manifest.write(&file, &payload.bytes).await;

            (FsResponse::SetState, None)
        }
        FsAction::GetState => {
            let file = FileIdentifier::Process(source.process.clone());

            match manifest.read(&file, None, None).await {
                Err(e) => return Err(e),
                Ok(bytes) => (FsResponse::GetState, Some(bytes)),
            }
        }
    };

    if expects_response.is_some() {
        let response = KernelMessage {
            id: id.clone(),
            source: Address {
                node: our_name.clone(),
                process: ProcessId::Name("filesystem".into()),
            },
            target: source.clone(),
            rsvp,
            message: Message::Response((
                Response {
                    ipc: Some(
                        serde_json::to_string::<Result<FsResponse, FileSystemError>>(&Ok(ipc))
                            .unwrap(),
                    ),
                    metadata, // for kernel
                },
                None,
            )),
            payload: Some(Payload {
                mime: None,
                bytes: bytes.unwrap_or_default(),
            }),
            signed_capabilities: None,
        };

        let _ = send_to_loop.send(response).await;
    }

    Ok(())
}

/// HELPERS

pub fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    for chunk in bytes.chunks(CHUNK_SIZE) {
        let chunk_hash: [u8; 32] = blake3::hash(chunk).into();
        hasher.update(&chunk_hash);
    }
    hasher.finalize().into()
}

async fn create_dir_if_dne(path: &str) -> Result<(), FileSystemError> {
    if let Err(_) = fs::read_dir(&path).await {
        match fs::create_dir_all(&path).await {
            Ok(_) => Ok(()),
            Err(e) => Err(FileSystemError::CouldNotMakeDir {
                path: path.into(),
                error: format!("{}", e),
            }),
        }
    } else {
        Ok(())
    }
}

fn make_error_message(
    our_name: String,
    id: u64,
    target: Address,
    error: FileSystemError,
) -> KernelMessage {
    KernelMessage {
        id,
        source: Address {
            node: our_name.clone(),
            process: ProcessId::Name("fileystem".into()),
        },
        target,
        rsvp: None,
        message: Message::Response((
            Response {
                ipc: Some(
                    serde_json::to_string::<Result<FileSystemResponse, FileSystemError>>(&Err(
                        error,
                    ))
                    .unwrap(),
                ),
                metadata: None,
            },
            None,
        )),
        payload: None,
        signed_capabilities: None,
    }
}
