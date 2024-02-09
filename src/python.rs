/// A python module that provides a python interface to Kinode processes.
/// This module is implemented in Rust using the PyO3 library.
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyString, PyTuple};

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use crate::types::*;
use std::process::Command;
use std::sync::Arc;
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

pub async fn python(
    our_node: String,
    send_to_loop: MessageSender,
    send_to_terminal: PrintSender,
    mut recv_from_loop: MessageReceiver,
    send_to_caps_oracle: CapMessageSender,
    home_directory_path: String,
) -> anyhow::Result<()> {
    let python_path = format!("{}/python", &home_directory_path);
    let vfs_path = format!("{}/vfs", &home_directory_path);
    println!("python: creating python dir: {}", python_path);

    if let Err(e) = fs::create_dir_all(&python_path).await {
        panic!("failed creating python dir! {:?}", e);
    }

    let mut process_queues: HashMap<ProcessId, Arc<Mutex<VecDeque<KernelMessage>>>> =
        HashMap::new();

    loop {
        tokio::select! {
            Some(km) = recv_from_loop.recv() => {
                if our_node.clone() != km.source.node {
                    println!(
                        "python: request must come from our_node={}, got: {}",
                        our_node,
                        km.source.node,
                    );
                    continue;
                }

                let queue = process_queues
                    .entry(km.source.process.clone())
                    .or_insert_with(|| Arc::new(Mutex::new(VecDeque::new())))
                    .clone();

                {
                    let mut queue_lock = queue.lock().await;
                    queue_lock.push_back(km.clone());
                }

                // clone Arcs
                let our_node = our_node.clone();
                let send_to_caps_oracle = send_to_caps_oracle.clone();
                let send_to_terminal = send_to_terminal.clone();
                let send_to_loop = send_to_loop.clone();
                let vfs_path = vfs_path.clone();

                tokio::spawn(async move {
                    let mut queue_lock = queue.lock().await;
                    if let Some(km) = queue_lock.pop_front() {
                        if let Err(e) = handle_request(
                            our_node.clone(),
                            km.clone(),
                            send_to_loop.clone(),
                            send_to_terminal.clone(),
                            send_to_caps_oracle.clone(),
                            vfs_path.clone(),
                        )
                        .await
                        {
                            let _ = send_to_loop
                                .send(make_error_message(our_node.clone(), &km, e))
                                .await;
                        }
                    }
                });
            }
        }
    }
}

async fn handle_request(
    our_node: String,
    km: KernelMessage,
    send_to_loop: MessageSender,
    send_to_terminal: PrintSender,
    send_to_caps_oracle: CapMessageSender,
    vfs_path: String,
) -> Result<(), PythonError> {
    let KernelMessage {
        id,
        source,
        message,
        lazy_load_blob: blob,
        ..
    } = km.clone();
    let Message::Request(Request {
        body,
        expects_response,
        metadata,
        ..
    }) = message.clone()
    else {
        return Err(PythonError::InputError {
            error: "not a request".into(),
        });
    };

    let request: PythonRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            println!("python: got invalid Request: {}", e);
            return Err(PythonError::InputError {
                error: "didn't serialize to PythonAction.".into(),
            });
        }
    };

    check_caps(
        our_node.clone(),
        source.clone(),
        send_to_caps_oracle.clone(),
        &request,
    )
    .await?;

    let package_path = PathBuf::from(format!("{}/{}", vfs_path, request.package_id));
    let (body, bytes) = match &request.action {
        PythonAction::RunScript { script, func, args } => {
            // get python path from .venv
            let venv_path = Path::new(".venv");
            let python_exec = format!("{}/bin/python", venv_path.display());
            let requirements_path = format!(
                "{}/pkg/scripts/requirements.txt",
                package_path.clone().display()
            );

            // install the requirements to the .venv
            let _ = Command::new("pip3")
                .arg("install")
                .arg("-r")
                .arg(&requirements_path.clone())
                .output()
                .unwrap();

            if Path::new(&python_exec).exists() {
                // load the string from the file
                let mut requirements_file = OpenOptions::new()
                    .read(true)
                    .write(false)
                    .create(false)
                    .truncate(false)
                    .open(&requirements_path)
                    .await
                    .map_err(|e| PythonError::IOError {
                        error: e.to_string(),
                    })
                    .unwrap();

                let mut requirements = String::new();
                requirements_file.read_to_string(&mut requirements).await?;

                // for each line in the file, install the requirements
                let _py = Python::with_gil(|py| -> PyResult<_> {
                    for line in requirements.lines() {
                        println!("python: line: {}", line);
                        // split the line by == and install the package
                        let mut line = line.split("==");
                        let package = line.next().unwrap();
                        py.import(package)?;
                    }
                    Ok(())
                });
            }

            // Load the script from the package's scripts directory
            let script_name = script.clone();
            let script_path = format!("{}/pkg/scripts/{}", package_path.display(), script);
            let mut script_file = OpenOptions::new()
                .read(true)
                .write(false)
                .create(false)
                .truncate(false)
                .open(&script_path)
                .await
                .map_err(|e| PythonError::IOError {
                    error: e.to_string(),
                })
                .unwrap();

            let mut script = String::new();
            script_file.read_to_string(&mut script).await?;

            let response = Python::with_gil(|py| -> PyResult<_> {
                // set the current working directory to the package's directory
                let os = PyModule::import(py, "os")?;
                println!("python: chdir to: {}", package_path.display());
                println!("python: args: {}", args.join(", "));
                os.call_method1("chdir", (format!("{}", package_path.display()),))?;

                let locals = [("os", py.import("os")?)].into_py_dict(py);
                let script_name = script_name.split('.').next().unwrap();

                let function_result: String = match py.run(&script, None, Some(locals)) {
                    Ok(_) => {
                        let module = PyModule::from_code(
                            py,
                            script.as_str(),
                            format!("{}.py", script_name).as_str(),
                            script_name,
                        )
                        .unwrap();

                        let function = module.getattr(func.as_str()).unwrap();
                        let py_args = PyTuple::new(
                            py,
                            &args
                                .iter()
                                .map(|arg| PyString::new(py, arg))
                                .collect::<Vec<_>>(),
                        );

                        let result = function.call1(py_args).unwrap();
                        result.str().unwrap().to_string()
                    }
                    Err(e) => {
                        println!("Failed to execute script: {:?}", e);
                        e.to_string()
                    }
                };
                Ok(function_result)
            });

            let result_data = match response {
                Ok(r) => PythonResponse::Result {
                    data: r.as_bytes().to_vec(),
                },
                Err(e) => {
                    return Err(PythonError::InputError {
                        error: format!("python: error running script: {}", e),
                    });
                }
            };

            (serde_json::to_vec(&result_data).unwrap(), None)
        }
    };

    if let Some(target) = km.rsvp.or_else(|| {
        expects_response.map(|_| Address {
            node: our_node.clone(),
            process: source.process.clone(),
        })
    }) {
        let response = KernelMessage {
            id,
            source: Address {
                node: our_node.clone(),
                process: PYTHON_PROCESS_ID.clone(),
            },
            target,
            rsvp: None,
            message: Message::Response((
                Response {
                    inherit: false,
                    body,
                    metadata,
                    capabilities: vec![],
                },
                None,
            )),
            lazy_load_blob: bytes.map(|bytes| LazyLoadBlob {
                mime: Some("application/octet-stream".into()),
                bytes,
            }),
        };

        let _ = send_to_loop.send(response).await;
    } else {
        send_to_terminal
            .send(Printout {
                verbosity: 2,
                content: format!(
                    "python: not sending response: {:?}",
                    serde_json::from_slice::<PythonResponse>(&body)
                ),
            })
            .await
            .unwrap();
    }

    Ok(())
}

async fn check_caps(
    our_node: String,
    source: Address,
    mut send_to_caps_oracle: CapMessageSender,
    request: &PythonRequest,
) -> Result<(), PythonError> {
    let (send_cap_bool, recv_cap_bool) = tokio::sync::oneshot::channel();
    let src_package_id = PackageId::new(source.process.package(), source.process.publisher());

    match &request.action {
        PythonAction::RunScript { script, .. } => {
            if src_package_id != request.package_id {
                return Err(PythonError::NoCap {
                    error: request.action.to_string(),
                });
            }
            add_capability(
                "write",
                &script,
                &our_node,
                &source,
                &mut send_to_caps_oracle,
            )
            .await?;
            send_to_caps_oracle
                .send(CapMessage::Has {
                    on: source.process.clone(),
                    cap: Capability {
                        issuer: Address {
                            node: our_node.clone(),
                            process: PYTHON_PROCESS_ID.clone(),
                        },
                        params: serde_json::to_string(&serde_json::json!({
                            "kind": "write",
                            "script": script,
                        }))
                        .unwrap(),
                    },
                    responder: send_cap_bool,
                })
                .await?;
            let has_cap = recv_cap_bool.await?;
            if !has_cap {
                return Err(PythonError::NoCap {
                    error: request.action.to_string(),
                });
            }
            Ok(())
        }
    }
}

async fn add_capability(
    kind: &str,
    script: &str,
    our_node: &str,
    source: &Address,
    send_to_caps_oracle: &mut CapMessageSender,
) -> Result<(), PythonError> {
    let cap = Capability {
        issuer: Address {
            node: our_node.to_string(),
            process: PYTHON_PROCESS_ID.clone(),
        },
        params: serde_json::to_string(&serde_json::json!({ "kind": kind, "script": script }))
            .unwrap(),
    };
    let (send_cap_bool, recv_cap_bool) = tokio::sync::oneshot::channel();
    send_to_caps_oracle
        .send(CapMessage::Add {
            on: source.process.clone(),
            caps: vec![cap],
            responder: send_cap_bool,
        })
        .await?;
    let _ = recv_cap_bool.await?;
    Ok(())
}

fn make_error_message(our_name: String, km: &KernelMessage, error: PythonError) -> KernelMessage {
    KernelMessage {
        id: km.id,
        source: Address {
            node: our_name.clone(),
            process: PYTHON_PROCESS_ID.clone(),
        },
        target: match &km.rsvp {
            None => km.source.clone(),
            Some(rsvp) => rsvp.clone(),
        },
        rsvp: None,
        message: Message::Response((
            Response {
                inherit: false,
                body: serde_json::to_vec(&PythonResponse::Err { error }).unwrap(),
                metadata: None,
                capabilities: vec![],
            },
            None,
        )),
        lazy_load_blob: None,
    }
}

impl std::fmt::Display for PythonAction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for PythonError {
    fn from(err: tokio::sync::oneshot::error::RecvError) -> Self {
        PythonError::NoCap {
            error: err.to_string(),
        }
    }
}

impl From<tokio::sync::mpsc::error::SendError<CapMessage>> for PythonError {
    fn from(err: tokio::sync::mpsc::error::SendError<CapMessage>) -> Self {
        PythonError::NoCap {
            error: err.to_string(),
        }
    }
}

impl From<std::io::Error> for PythonError {
    fn from(err: std::io::Error) -> Self {
        PythonError::IOError {
            error: err.to_string(),
        }
    }
}
