// use raphtory::prelude::*;
// use std::any::Any;

use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::opt::Config;
use surrealdb::{Error, Response, Surreal};

pub type SurrealDBConn = Surreal<Db>;

#[tokio::main]
async fn main() {
    let db = SurrealDBConn::new::<RocksDb>(("/tmp/surreal_test", Config::default()))
        .await
        .unwrap();

    db.query("DEFINE namespace test;").await.unwrap();
    db.use_ns("test").await.unwrap();

    db.query("DEFINE DATABASE test;").await.unwrap();
    db.use_db("test").await.unwrap();
    db.query("DEFINE TABLE person;").await.unwrap();

    let mut create_res = db.query("CREATE person SET name = 'Tobie', company = 'SurrealDB', skills = ['Rust', 'Go', 'JavaScript'];").await.unwrap();
    let create_results: surrealdb::sql::Value = create_res.take(0).unwrap();

    println!("create_results = {:?}", create_results.as_raw_string());

    let res: Result<Response, Error> = db.query("SELECT * FROM person;").await;
    let mut results = match res {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };
    let results: surrealdb::sql::Value = results.take(0).unwrap();
    println!("select_results = {:?}", results.as_raw_string());
}
