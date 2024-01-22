use kinode_process_lib::{await_next_request_body, call_init, println, Address};

wit_bindgen::generate!({
    path: "../../../wit",
    world: "process",
    exports: {
        world: Component,
    },
});

call_init!(init);

fn init(_our: Address) {
    let Ok(args) = await_next_request_body() else {
        println!("echo: failed to get args, aborting");
        return;
    };

    println!(
        "{}",
        String::from_utf8(args).unwrap_or("echo: error".into())
    );
}
