use serde::{Deserialize, Serialize};

use kinode_process_lib::{await_message, call_init, graphdb, println, Address, Message, Response};

wit_bindgen::generate!({
    path: "wit",
    world: "process",
    exports: {
        world: Component,
    },
});

///  Example usage:
///
///  /m our@graphdb_test:graphdb_test:template.os {"Open": {"db": "test_db"}}
///
///  /m our@graphdb_test:graphdb_test:template.os {"Read": {"db": "test_db", "query": "SELECT * FROM person;"}}
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
        } => {
            match serde_json::from_slice(body)? {
                TestRequest::Open { ref db } => {
                    println!("graphdb_test: db opening...");
                    let db = graphdb::open(our.package_id(), db)?;
                    println!("graphdb_test: db opened");
                    // Define the table
                    let define_res = db.define(graphdb::DefineResourceType::Table {
                        name: "person".into(),
                    });
                    println!("graphdb_test: db.define {:?}", define_res);

                    // Create a person
                    db.statement(
                        "CREATE person SET name = $name, company = $company;".into(),
                        Some(vec![
                            ("name".into(), "John Doe".into()),
                            ("company".into(), "Acme".into()),
                        ]),
                    )?;
                    println!("graphdb_test: db.create(person)");
                    Response::new()
                        .body(serde_json::to_vec(&TestResponse::DbOpened).unwrap())
                        .send()
                        .unwrap();
                }
                TestRequest::Read { ref db, ref query } => {
                    let db = graphdb::open(our.package_id(), db)?;

                    let result = db.read(query.into())?;
                    // convert results to a json array

                    let result = serde_json::to_value(result)?;

                    println!("graphdb_test: db.read {}", result.to_string());

                    Response::new()
                        .body(serde_json::to_vec(&result).unwrap())
                        .send()
                        .unwrap();
                }
            }
        }
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
