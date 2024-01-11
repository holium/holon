#![feature(let_chains)]
use chess::{Board as AiBoard, ChessMove};
use nectar_process_lib::{
    await_message, call_init, get_blob, get_typed_state, http, println, save_capabilities,
    set_state, our_capabilities, Address, Capability, LazyLoadBlob, Message, NodeId, ProcessId, Request, Response,
};
use pleco::Board;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};

mod llm_types;
extern crate base64;

//
// Our "chess protocol" request/response format. We'll always serialize these
// to a byte vector and send them over IPC.
//

#[derive(Debug, Serialize, Deserialize)]
enum ChessRequest {
    NewGame {
        white: String,
        black: String,
    },
    /// Name of the model you want to play with. Must exist within your LLM manager app!
    NewGameAI(String),
    Move {
        game_id: String,
        move_str: String,
    },
    Resign(String),
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
enum ChessResponse {
    NewGameAccepted,
    NewGameRejected,
    MoveAccepted,
    MoveRejected,
}

//
// Our serializable state format.
//

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Game {
    pub id: String,              // the node with whom we are playing
    pub ai_game: Option<String>, // None = not AI, String is move history for the prompt
    pub turns: u64,
    pub board: String,
    pub white: String,
    pub black: String,
    pub ended: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChessState {
    pub games: HashMap<String, Game>, // game is by opposing player id
    pub clients: HashSet<u32>,        // doesn't get persisted
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredChessState {
    pub games: HashMap<String, Game>, // game is by opposing player id
}

fn save_chess_state(state: &ChessState) {
    set_state(&bincode::serialize(&state.games).unwrap());
}

fn load_chess_state() -> ChessState {
    match get_typed_state(|bytes| Ok(bincode::deserialize::<HashMap<String, Game>>(bytes)?)) {
        Some(games) => ChessState {
            games,
            clients: HashSet::new(),
        },
        None => ChessState {
            games: HashMap::new(),
            clients: HashSet::new(),
        },
    }
}

fn send_ws_update(our: &Address, game: &Game, open_channels: &HashSet<u32>) -> anyhow::Result<()> {
    for channel in open_channels {
        Request::new()
            .target((&our.node, "http_server", "sys", "nectar"))
            .body(serde_json::to_vec(
                &http::HttpServerAction::WebSocketPush {
                    channel_id: *channel,
                    message_type: http::WsMessageType::Binary,
                },
            )?)
            .blob(LazyLoadBlob {
                mime: Some("application/json".to_string()),
                bytes: serde_json::json!({
                    "kind": "game_update",
                    "data": game,
                })
                .to_string()
                .into_bytes(),
            })
            .send()?;
    }
    Ok(())
}

/// Fire a request to the given AI model in the LLM manager app, asking it to make a move.
/// Await its response and apply it to the game.
fn request_ai_move(our: &Address, game: &mut Game) -> anyhow::Result<()> {
    println!("Requesting AI move...");
    let mut prompt = format!(
        // r#"[Event "FIDE World Championship Match 2024"]
        // [Site "Los Angeles, USA"]
        // [Date "2024.12.01"]
        // [Round "5"]
        // [White "Carlsen, Magnus"]
        // [Black "Nepomniachtchi, Ian"]
        // [Result "1-0"]
        // [WhiteElo "2885"]
        // [WhiteTitle "GM"]
        // [WhiteFideId "1503014"]
        // [BlackElo "2812"]
        // [BlackTitle "GM"]
        // [BlackFideId "4168119"]
        // [TimeControl "40/7200:20/3600:900+30"]
        // [UTCDate "2024.11.27"]
        // [UTCTime "09:01:25"]
        // [Variant "Standard"]
        
        // 1.{}"#,
        // game.ai_game.as_ref().unwrap(),
        "here is a chess board in FEN notation: \"{}\" you are {}. Make a legal chess move. Do not put out any additional text, simply respond with the move encoded as a SAN move.",
        game.board,
        if game.white == game.id { "white" } else { "black" }
    );
    // println!("prompt: {}", prompt);
    loop {
        let Ok(Message::Response { body, .. }) = Request::new()
            .target((
                our.node.as_str(),
                format!("{}:llm:nectar", game.id)
                    .parse::<ProcessId>()
                    .unwrap(),
            ))
            .body(
                json!({"Chat": {
                    "prompt": prompt,
                    "params": {
                        "max_tokens": 4,
                        "temperature": 1,// TODO should be 0
                    }}
                })
                .to_string()
                .into_bytes(),
            )
            .capabilities(our_capabilities()) // TODO too many
            .send_and_await_response(10)?
        else {
            return Err(anyhow::anyhow!(
                "AI did not respond properly to chat request!"
            ));
        };
        let response = serde_json::from_slice::<llm_types::ChatResponse>(&body)?;
        let mut board = AiBoard::from_fen(game.board.clone()).unwrap();
        println!("raw response: {}", response.content.replace(" ", ""));
        match ChessMove::from_san(&board, &response.content.replace(" ", "")) {
            Ok(valid_move) => {
                println!("Got AI move: {valid_move}");
                board.make_move_new(valid_move);
                game.board = board.to_string();
                return Ok(());
            }
            Err(e) => {
                // Reject invalid moves!
                // TODO reprompt could be a lot better
                println!(
                    "chess: ai gave us an illegal move! {}",
                    response.content.replace(" ", "")
                );
                continue;
            }
        }
    }
}

// Boilerplate: generate the wasm bindings for an Uqbar app
wit_bindgen::generate!({
    path: "../../../wit",
    world: "process",
    exports: {
        world: Component,
    },
});
// After generating bindings, use this macro to define the Component struct
// and its init() function, which the kernel will look for on startup.
call_init!(initialize);

fn initialize(our: Address) {
    // A little printout to show in terminal that the process has started.
    println!("{} by {}: start", our.process(), our.publisher());

    // Serve the index.html and other UI files found in pkg/ui at the root path.
    http::serve_ui(&our, "ui").unwrap();

    // Allow HTTP requests to be made to /games; they will be handled dynamically.
    http::bind_http_path("/games", true, false).unwrap();

    // Allow websockets to be opened at / (our process ID will be prepended).
    http::bind_ws_path("/", true, false).unwrap();

    // Grab our state, then enter the main event loop.
    let mut state: ChessState = load_chess_state();
    main_loop(&our, &mut state);
}

fn main_loop(our: &Address, state: &mut ChessState) {
    loop {
        // Call await_message() to wait for any incoming messages.
        // If we get a network error, make a print and throw it away.
        // In a high-quality consumer-grade app, we'd want to explicitly handle
        // this and surface it to the user.
        match await_message() {
            Err(send_error) => {
                println!("{our}: got network error: {send_error:?}");
                continue;
            }
            Ok(message) => match handle_request(&our, &message, state) {
                Ok(()) => continue,
                Err(e) => println!("{our}: error handling request: {:?}", e),
            },
        }
    }
}

/// Handle chess protocol messages from ourself *or* other nodes.
fn handle_request(our: &Address, message: &Message, state: &mut ChessState) -> anyhow::Result<()> {
    // Throw away responses. We never expect any responses *here*, because for every
    // chess protocol request, we *await* its response in-place. This is appropriate
    // for direct node<>node comms, less appropriate for other circumstances...
    if !message.is_request() {
        return Ok(());
    }
    // If the request is from another node, handle it as an incoming request.
    // Note that we can enforce the ProcessId as well, but it shouldn't be a trusted
    // piece of information, since another node can easily spoof any ProcessId on a request.
    // It can still be useful simply as a protocol-level switch to handle different kinds of
    // requests from the same node, with the knowledge that the remote node can finagle with
    // which ProcessId a given message can be from. It's their code, after all.
    if message.source().node != our.node {
        // Deserialize the request IPC to our format, and throw it away if it
        // doesn't fit.
        let Ok(chess_request) = serde_json::from_slice::<ChessRequest>(message.body()) else {
            return Err(anyhow::anyhow!("invalid chess request"));
        };
        handle_chess_request(our, &message.source().node, state, &chess_request)
    // ...and if the request is from ourselves, handle it as our own!
    // Note that since this is a local request, we *can* trust the ProcessId.
    // Here, we'll accept messages from the local terminal so as to make this a "CLI" app.
    } else if message.source().node == our.node
        && message.source().process == "terminal:terminal:nectar"
    {
        let Ok(chess_request) = serde_json::from_slice::<ChessRequest>(message.body()) else {
            return Err(anyhow::anyhow!("invalid chess request"));
        };
        handle_local_request(our, state, &chess_request)
    } else if message.source().node == our.node
        && message.source().process == "http_server:sys:nectar"
    {
        // receive HTTP requests and websocket connection messages from our server
        match serde_json::from_slice::<http::HttpServerRequest>(message.body())? {
            http::HttpServerRequest::Http(ref incoming) => {
                match handle_http_request(our, state, incoming) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        println!("chess: error handling http request: {:?}", e);
                        http::send_response(
                            http::StatusCode::SERVICE_UNAVAILABLE,
                            None,
                            "Service Unavailable".to_string().as_bytes().to_vec(),
                        )
                    }
                }
            }
            http::HttpServerRequest::WebSocketOpen {
                path: _,
                channel_id,
            } => {
                // We know this is authenticated and unencrypted because we only
                // bound one path, the root path. So we know that client
                // frontend opened a websocket and can send updates
                state.clients.insert(channel_id);
                Ok(())
            }
            http::HttpServerRequest::WebSocketClose(channel_id) => {
                // client frontend closed a websocket
                state.clients.remove(&channel_id);
                Ok(())
            }
            http::HttpServerRequest::WebSocketPush { .. } => {
                // client frontend sent a websocket message
                // we don't expect this! we only use websockets to push updates
                Ok(())
            }
        }
    } else {
        // If we get a request from ourselves that isn't from the terminal, we'll just
        // throw it away. This is a good place to put a printout to show that we've
        // received a request from ourselves that we don't know how to handle.
        return Err(anyhow::anyhow!(
            "got request from not-the-terminal, ignoring"
        ));
    }
}

/// Handle chess protocol messages from other nodes.
fn handle_chess_request(
    our: &Address,
    source_node: &NodeId,
    state: &mut ChessState,
    action: &ChessRequest,
) -> anyhow::Result<()> {
    println!("chess: handling action from {source_node}: {action:?}");

    // For simplicity's sake, we'll just use the node we're playing with as the game id.
    // This limits us to one active game per partner.
    let game_id = source_node;

    match action {
        ChessRequest::NewGame { white, black } => {
            // Make a new game with source.node
            // This will replace any existing game with source.node!
            if state.games.contains_key(game_id) {
                println!("chess: resetting game with {game_id} on their request!");
            }
            let game = Game {
                id: game_id.to_string(),
                ai_game: None,
                turns: 0,
                board: Board::start_pos().fen(),
                white: white.to_string(),
                black: black.to_string(),
                ended: false,
            };
            // Use our helper function to persist state after every action.
            // The simplest and most trivial way to keep state. You'll want to
            // use a database or something in a real app, and consider performance
            // when doing intensive data-based operations.
            send_ws_update(&our, &game, &state.clients)?;
            state.games.insert(game_id.to_string(), game);
            save_chess_state(&state);
            // Send a response to tell them we've accepted the game.
            // Remember, the other player is waiting for this.
            Response::new()
                .body(serde_json::to_vec(&ChessResponse::NewGameAccepted)?)
                .send()
        }
        ChessRequest::NewGameAI(_) => {
            // Ignore requests for an AI game from other nodes, that's not how the
            // protocol works.
            return Err(anyhow::anyhow!("AI is a local-only feature!"));
        }
        ChessRequest::Move { ref move_str, .. } => {
            // Get the associated game, and respond with an error if
            // we don't have it in our state.
            let Some(game) = state.games.get_mut(game_id) else {
                // If we don't have a game with them, reject the move.
                return Response::new()
                    .body(serde_json::to_vec(&ChessResponse::MoveRejected)?)
                    .send();
            };
            // Convert the saved board to one we can manipulate.
            let mut board = Board::from_fen(&game.board).unwrap();
            if !board.apply_uci_move(move_str) {
                // Reject invalid moves!
                return Response::new()
                    .body(serde_json::to_vec(&ChessResponse::MoveRejected)?)
                    .send();
            }
            game.turns += 1;
            if board.checkmate() || board.stalemate() {
                game.ended = true;
            }
            // Persist state.
            game.board = board.fen();
            send_ws_update(&our, &game, &state.clients)?;
            save_chess_state(&state);
            // Send a response to tell them we've accepted the move.
            Response::new()
                .body(serde_json::to_vec(&ChessResponse::MoveAccepted)?)
                .send()
        }
        ChessRequest::Resign(_) => {
            // They've resigned. The sender isn't waiting for a response to this,
            // so we don't need to send one.
            match state.games.get_mut(game_id) {
                Some(game) => {
                    game.ended = true;
                    send_ws_update(&our, &game, &state.clients)?;
                    save_chess_state(&state);
                }
                None => {}
            }
            Ok(())
        }
    }
}

/// Handle actions we are performing. Here's where we'll send_and_await various requests.
fn handle_local_request(
    our: &Address,
    state: &mut ChessState,
    action: &ChessRequest,
) -> anyhow::Result<()> {
    match action {
        ChessRequest::NewGame { white, black } => {
            // Create a new game. We'll enforce that one of the two players is us.
            if white != &our.node && black != &our.node {
                return Err(anyhow::anyhow!("cannot start a game without us!"));
            }
            let game_id = if white == &our.node { black } else { white };
            // If we already have a game with this player, throw an error.
            if let Some(game) = state.games.get(game_id)
                && !game.ended
            {
                return Err(anyhow::anyhow!("already have a game with {game_id}"));
            };
            // Send the other player a NewGame request
            // The request is exactly the same as what we got from terminal.
            // We'll give them 5 seconds to respond...
            let Ok(Message::Response { ref body, .. }) = Request::new()
                .target((game_id.as_ref(), our.process.clone()))
                .body(serde_json::to_vec(&action)?)
                .send_and_await_response(5)?
            else {
                return Err(anyhow::anyhow!(
                    "other player did not respond properly to new game request"
                ));
            };
            // If they accept, create a new game -- otherwise, error out.
            if serde_json::from_slice::<ChessResponse>(body)? != ChessResponse::NewGameAccepted {
                return Err(anyhow::anyhow!("other player rejected new game request!"));
            }
            // New game with default board.
            let game = Game {
                id: game_id.to_string(),
                ai_game: None,
                turns: 0,
                board: Board::start_pos().fen(),
                white: white.to_string(),
                black: black.to_string(),
                ended: false,
            };
            state.games.insert(game_id.to_string(), game);
            save_chess_state(&state);
            Ok(())
        }
        ChessRequest::NewGameAI(model_name) => {
            // Create a new game with the AI.
            // First we need the capability from the LLM manager app.
            let model_address: Address = model_name.parse().unwrap();
            let model_name: String = model_address.process.process_name.clone();
            let res = Request::new()
                .target((&model_address.node, "main", "llm", "nectar"))
                .body(
                    json!({ "RequestAccess": { "model": model_address.process.process_name.clone(), "quantity": 5 } })
                    .to_string()
                    .into_bytes(),
                )
                .send_and_await_response(5)??;
            // This includes both the messaging capability and the model capability.
            save_capabilities(res.capabilities());
            // Then, make a new game, picking randomly between AI and player as white.
            let (white, black) = if rand::random() {
                (model_name.as_str(), our.node.as_str())
            } else {
                (our.node.as_str(), model_name.as_str())
            };
            let mut game = Game {
                id: model_name.to_string(),
                ai_game: Some(String::new()),
                turns: 0,
                board: Board::start_pos().fen(),
                white: white.to_string(),
                black: black.to_string(),
                ended: false,
            };
            // Lastly, if it's the AI's turn, send a request that gives it the current board
            // and asks it to make a move!
            if white == model_name {
                request_ai_move(our, &mut game)?;
                game.turns += 1;
            }
            state.games.insert(model_name.to_string(), game);
            save_chess_state(&state);
            Ok(())
        }
        ChessRequest::Move { game_id, move_str } => {
            // Make a move. We'll enforce that it's our turn. The game_id is the
            // person we're playing with.
            let Some(mut game) = state.games.get_mut(game_id) else {
                return Err(anyhow::anyhow!("no game with {game_id}"));
            };
            if (game.turns % 2 == 0 && game.white != our.node)
                || (game.turns % 2 == 1 && game.black != our.node)
            {
                return Err(anyhow::anyhow!("not our turn!"));
            } else if game.ended {
                return Err(anyhow::anyhow!("that game is over!"));
            }
            let mut board = Board::from_fen(&game.board).unwrap();
            if !board.apply_uci_move(move_str) {
                return Err(anyhow::anyhow!("illegal move!"));
            }
            game.board = board.fen();
            game.turns += 1;
            if board.checkmate() || board.stalemate() {
                game.ended = true;
            }
            // If this is an AI game, we'll send a request to the AI to make a move.
            // Otherwise, we'll send a request to the other player to make a move.
            if game.ai_game.is_some() && !game.ended {
                request_ai_move(our, &mut game)?;
                // update the game
                game.turns += 1;
                if board.checkmate() || board.stalemate() {
                    game.ended = true;
                }
            } else {
                // Send the move to the other player, then check if the game is over.
                // The request is exactly the same as what we got from terminal.
                // We'll give them 5 seconds to respond...
                let Ok(Message::Response { ref body, .. }) = Request::new()
                    .target((game_id.as_ref(), our.process.clone()))
                    .body(serde_json::to_vec(&action)?)
                    .send_and_await_response(5)?
                else {
                    return Err(anyhow::anyhow!(
                        "other player did not respond properly to our move"
                    ));
                };
                if serde_json::from_slice::<ChessResponse>(body)? != ChessResponse::MoveAccepted {
                    return Err(anyhow::anyhow!("other player rejected our move"));
                }
            }
            save_chess_state(&state);
            Ok(())
        }
        ChessRequest::Resign(ref with_who) => {
            // Resign from a game with a given player.
            let Some(game) = state.games.get_mut(with_who) else {
                return Err(anyhow::anyhow!("no game with {with_who}"));
            };
            // send the other player an end game request -- no response expected
            Request::new()
                .target((with_who.as_ref(), our.process.clone()))
                .body(serde_json::to_vec(&action)?)
                .send()?;
            game.ended = true;
            save_chess_state(&state);
            Ok(())
        }
    }
}

/// Handle HTTP requests from our own frontend.
fn handle_http_request(
    our: &Address,
    state: &mut ChessState,
    http_request: &http::IncomingHttpRequest,
) -> anyhow::Result<()> {
    if http_request.path()? != "games" {
        return http::send_response(
            http::StatusCode::NOT_FOUND,
            None,
            "Not Found".to_string().as_bytes().to_vec(),
        );
    }
    match http_request.method.as_str() {
        // on GET: give the frontend all of our active games
        "GET" => http::send_response(
            http::StatusCode::OK,
            Some(HashMap::from([(
                String::from("Content-Type"),
                String::from("application/json"),
            )])),
            serde_json::to_vec(&state.games)?,
        ),
        // on POST: create a new game
        "POST" => {
            let Some(blob) = get_blob() else {
                return http::send_response(http::StatusCode::BAD_REQUEST, None, vec![]);
            };
            let blob_json = serde_json::from_slice::<serde_json::Value>(&blob.bytes)?;
            let Some(game_id) = blob_json["id"].as_str() else {
                return http::send_response(http::StatusCode::BAD_REQUEST, None, vec![]);
            };
            if let Some(game) = state.games.get(game_id)
                && !game.ended
            {
                return http::send_response(http::StatusCode::CONFLICT, None, vec![]);
            };

            let player_white = blob_json["white"]
                .as_str()
                .unwrap_or(our.node.as_str())
                .to_string();
            let player_black = blob_json["black"].as_str().unwrap_or(game_id).to_string();

            // send the other player a new game request
            let Ok(msg) = Request::new()
                .target((game_id, our.process.clone()))
                .body(serde_json::to_vec(&ChessRequest::NewGame {
                    white: player_white.clone(),
                    black: player_black.clone(),
                })?)
                .send_and_await_response(5)?
            else {
                return Err(anyhow::anyhow!(
                    "other player did not respond properly to new game request"
                ));
            };
            // if they accept, create a new game
            // otherwise, should surface error to FE...
            if serde_json::from_slice::<ChessResponse>(msg.body())?
                != ChessResponse::NewGameAccepted
            {
                return Err(anyhow::anyhow!("other player rejected new game request"));
            }
            // create a new game
            let game = Game {
                id: game_id.to_string(),
                ai_game: None,
                turns: 0,
                board: Board::start_pos().fen(),
                white: player_white,
                black: player_black,
                ended: false,
            };
            let body = serde_json::to_vec(&game)?;
            state.games.insert(game_id.to_string(), game);
            save_chess_state(&state);
            http::send_response(
                http::StatusCode::OK,
                Some(HashMap::from([(
                    String::from("Content-Type"),
                    String::from("application/json"),
                )])),
                body,
            )
        }
        // on PUT: make a move
        "PUT" => {
            let Some(blob) = get_blob() else {
                return http::send_response(http::StatusCode::BAD_REQUEST, None, vec![]);
            };
            let blob_json = serde_json::from_slice::<serde_json::Value>(&blob.bytes)?;
            let Some(game_id) = blob_json["id"].as_str() else {
                return http::send_response(http::StatusCode::BAD_REQUEST, None, vec![]);
            };
            let Some(mut game) = state.games.get_mut(game_id) else {
                return http::send_response(http::StatusCode::NOT_FOUND, None, vec![]);
            };
            if (game.turns % 2 == 0 && game.white != our.node)
                || (game.turns % 2 == 1 && game.black != our.node)
            {
                return http::send_response(http::StatusCode::FORBIDDEN, None, vec![]);
            } else if game.ended {
                return http::send_response(http::StatusCode::CONFLICT, None, vec![]);
            }
            let Some(move_str) = blob_json["move"].as_str() else {
                return http::send_response(http::StatusCode::BAD_REQUEST, None, vec![]);
            };
            let mut board = Board::from_fen(&game.board).unwrap();
            if !board.apply_uci_move(move_str) {
                // TODO surface illegal move to player or something here
                return http::send_response(http::StatusCode::BAD_REQUEST, None, vec![]);
            }
            game.board = board.fen();
            game.turns += 1;
            if board.checkmate() || board.stalemate() {
                game.ended = true;
            }
            // If this is an AI game, we'll send a request to the AI to make a move.
            // Otherwise, we'll send a request to the other player to make a move.
            if game.ai_game.is_some() && !game.ended {
                request_ai_move(our, &mut game)?;
                // update the game
                game.turns += 1;
                if board.checkmate() || board.stalemate() {
                    game.ended = true;
                }
            } else {
                // Send the move to the other player.
                let Ok(msg) = Request::new()
                    .target((game_id, our.process.clone()))
                    .body(serde_json::to_vec(&ChessRequest::Move {
                        game_id: game_id.to_string(),
                        move_str: move_str.to_string(),
                    })?)
                    .send_and_await_response(5)?
                else {
                    return Err(anyhow::anyhow!(
                        "other player did not respond properly to our move"
                    ));
                };
                if serde_json::from_slice::<ChessResponse>(msg.body())?
                    != ChessResponse::MoveAccepted
                {
                    return Err(anyhow::anyhow!("other player rejected our move"));
                }
            }
            send_ws_update(&our, &game, &state.clients)?;
            // update state and return to FE
            let body = serde_json::to_vec(&game)?;
            save_chess_state(&state);
            // return the game
            http::send_response(
                http::StatusCode::OK,
                Some(HashMap::from([(
                    String::from("Content-Type"),
                    String::from("application/json"),
                )])),
                body,
            )
        }
        // on DELETE: end the game
        "DELETE" => {
            let Some(game_id) = http_request.query_params.get("id") else {
                return http::send_response(http::StatusCode::BAD_REQUEST, None, vec![]);
            };
            let Some(game) = state.games.get_mut(game_id) else {
                return http::send_response(http::StatusCode::BAD_REQUEST, None, vec![]);
            };
            // send the other player an end game request
            Request::new()
                .target((game_id.as_str(), our.process.clone()))
                .body(serde_json::to_vec(&ChessRequest::Resign(our.node.clone()))?)
                .send()?;
            game.ended = true;
            let body = serde_json::to_vec(&game)?;
            save_chess_state(&state);
            http::send_response(
                http::StatusCode::OK,
                Some(HashMap::from([(
                    String::from("Content-Type"),
                    String::from("application/json"),
                )])),
                body,
            )
        }
        // Any other method will be rejected.
        _ => http::send_response(http::StatusCode::METHOD_NOT_ALLOWED, None, vec![]),
    }
}
