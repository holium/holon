use std::collections::HashMap;

use kinode_process_lib::{
    await_message, call_init, get_blob,
    http::{
        bind_http_path, send_response, serve_ui, HttpServerRequest, IncomingHttpRequest,
        StatusCode,
    },
    println, Address, ProcessId, Message,
};
use serde::{Deserialize, Serialize};

mod data;
use data::{MEME_CATEGORIES, MEME_TEMPLATES, MEMES};

struct Component;

impl Guest for Component {
    fn init(our: String) {
        let our: Address = our.parse().expect("Failed to parse address");
        if let Err(e) = main(our) {
            println!("homepage: ended with error: {:?}", e);
        }
    }
}

wit_bindgen::generate!({
    path: "wit",
    world: "process",
    exports: {
        world: Component,
    },
});

fn handle_http_server_request(
    source: &Address,
    body: &[u8],
) -> anyhow::Result<()> {
    let server_request = serde_json::from_slice::<HttpServerRequest>(body)
        .map_err(|e| {
            println!("Failed to parse server request: {:?}", e);
            e
        })?;

    match server_request {
        HttpServerRequest::Http(request) => {
            match request.method()?.as_str() {
                "GET" => {
                    let mut headers = HashMap::new();
                    headers.insert("Content-Type".to_string(), "application/json".to_string());

                    // Route to appropriate endpoint based on the path
                    match request.path()?.as_str() {
                        "/categories" => {
                            send_response(
                                StatusCode::OK,
                                Some(headers.clone()),
                                serde_json::to_vec(&*MEME_CATEGORIES)?,
                            )?;
                        }
                        "/templates" => {
                            send_response(
                                StatusCode::OK,
                                Some(headers.clone()),
                                serde_json::to_vec(&*MEME_TEMPLATES)?,
                            )?;
                        }
                        "/memes" => {
                            send_response(
                                StatusCode::OK,
                                Some(headers),
                                serde_json::to_vec(&*MEMES)?, // Assume MEMES is a constant holding your memes data
                            )?;
                        }
                        _ => {
                            send_response(StatusCode::NOT_FOUND, None, vec![])?;
                        }
                    }
                }
                _ => {
                    send_response(StatusCode::METHOD_NOT_ALLOWED, None, vec![])?;
                }
            }
        }
        _ => {
            println!("Ignored non-HTTP server request");
        }
    };

    Ok(())
}

fn main(our: Address) -> anyhow::Result<()> {
    println!("Server started");

    serve_ui(&our, "ui")?;
    bind_http_path("/categories", true, false)?;
    bind_http_path("/templates", true, false)?;
    bind_http_path("/memes", true, false)?; // Bind the new /memes endpoint

    main_loop(&our);

    Ok(())
}

fn main_loop(our: &Address) {
    loop {
        match await_message() {
            Err(send_error) => {
                println!("{our}: got network error: {send_error:?}");
                continue;
            }
            Ok(message) => match handle_request(&our, &message) {
                Ok(()) => continue,
                Err(e) => println!("{our}: error handling request: {:?}", e),
            },
        }
    }
}

fn handle_request(our: &Address, message: &Message) -> anyhow::Result<()> {
    if message.source().node == our.node && message.source().process == "http_server:distro:sys"
    {
        handle_http_server_request(&our, &message.body())?;
    }

    Ok(())
}
