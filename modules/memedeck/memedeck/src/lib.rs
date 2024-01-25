use std::collections::HashMap;

use kinode_process_lib::{
    await_message, call_init, get_blob, http,
    http::{
        bind_http_path, send_response, serve_ui, HttpServerRequest, IncomingHttpRequest,
        StatusCode,
    },
    println, Address, ProcessId, Message,
};
use serde::{Deserialize, Serialize};

mod data;
use data::{Meme, MEME_CATEGORIES, MEME_TEMPLATES, MEMES, UploadData};

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
                            println!("Got request for memes");
                            println!("request: {:?}", request);
                            // TO-FIX: field `query_params` of struct `IncomingHttpRequest` is private
                            // let query_params = request.query_params;
                            // let query = parse_query_param(&query_params, "q");
                            let query = None;
                            let memes = filter_memes_by_query(&query);
                            let mut headers = HashMap::new();
                            headers.insert("Content-Type".to_string(), "application/json".to_string());
                            
                            send_response(
                                StatusCode::OK,
                                Some(headers),
                                serde_json::to_vec(&memes)?,
                            )?;
                        }
                        _ => {
                            send_response(StatusCode::NOT_FOUND, None, vec![])?;
                        }
                    }
                }
                "POST" => {
                    match request.path()?.as_str() {
                        "/upload" => {
                            // Extract the URL from the request body
                            let Some(blob) = get_blob() else {
                                return http::send_response(http::StatusCode::BAD_REQUEST, None, vec![]);
                            };
                            let blob_json = serde_json::from_slice::<serde_json::Value>(&blob.bytes)?;
                            let upload_data = serde_json::from_value::<UploadData>(blob_json)?;

                            // TODO: Handle the URL (e.g., download or process the data)
                            println!("Received URL for upload: {}", upload_data.url);

                            // Send a success response
                            send_response(StatusCode::OK, None, vec![])?;
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

fn filter_memes_by_query(query: &Option<String>) -> Vec<Meme> {
    if let Some(query) = query {
        MEMES
            .iter()
            .filter(|meme| meme.matches_query(&query))
            .cloned()
            .collect()
    } else {
        MEMES.clone()
    }
}

fn parse_query_param(query_params: &str, param_name: &str) -> Option<String> {
    query_params
        .split('&')
        .filter_map(|param| {
            let parts: Vec<&str> = param.split('=').collect();
            if parts.len() == 2 && parts[0] == param_name {
                Some(parts[1].to_string())
            } else {
                None
            }
        })
        .next()
}

impl Meme {
    fn matches_query(&self, query: &str) -> bool {
        // TODO: make more sophisticated
        if self.id.contains(query) {
            return true;
        }

        return false;
    }
}

fn main(our: Address) -> anyhow::Result<()> {
    println!("Server started");

    serve_ui(&our, "ui")?;
    bind_http_path("/categories", true, false)?;
    bind_http_path("/templates", true, false)?;
    bind_http_path("/memes", true, false)?; // TODO: handle search query
    bind_http_path("/upload", true, false)?;


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
