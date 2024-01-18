use serde::{Deserialize, Serialize};

use kinode_process_lib::{await_message, call_init, graphdb, println, Address, Message, Response};

wit_bindgen::generate!({
    path: "wit",
    world: "process",
    exports: {
        world: Component,
    },
});

#[derive(Debug, Serialize, Deserialize)]
enum TestRequest {
    Open { db: String },
    Read { db: String, query: String },
}

#[derive(Debug, Serialize, Deserialize)]
enum TestResponse {
    DbOpened,
}

fn handle_message(our: &Address) -> anyhow::Result<()> {
    let message = await_message()?;

    match message {
        Message::Response { .. } => {
            return Err(anyhow::anyhow!("unexpected Response: {:?}", message));
        }
        Message::Request {
            // ref source,
            ref body,
            ..
        } => match serde_json::from_slice(body)? {
            TestRequest::Open { ref db } => {
                let db = graphdb::open(our.package_id(), db)?;
                println!("graphdb_test: db opened");

                db.create("CREATE person SET name = 'Tobie', company = 'SurrealDB', skills = ['Rust', 'Go', 'JavaScript'];".into(), vec![])?;
                println!("graphdb_test: db.create(person)");
                Response::new()
                    .body(serde_json::to_vec(&TestResponse::DbOpened).unwrap())
                    .send()
                    .unwrap();
            }
            TestRequest::Read { ref db, ref query } => {
                let db = graphdb::open(our.package_id(), db)?;

                let result = db.read(query.into(), vec![])?;
                // convert results to a json array

                let result = serde_json::to_value(result)?;

                println!("graphdb_test: db.read {}", result.to_string());

                Response::new()
                    .body(serde_json::to_vec(&result).unwrap())
                    .send()
                    .unwrap();
            }
        },
    }
    Ok(())
}

call_init!(init);

fn init(our: Address) {
    println!("graphdb_test: begin");

    loop {
        match handle_message(&our) {
            Ok(()) => {}
            Err(e) => {
                println!("graphdb_test: error: {:?}", e);
            }
        };
    }
}
