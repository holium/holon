use bindings::component::microkernel_process::types::WitPayload;
use bindings::component::microkernel_process::types::WitProtomessage;
use bindings::component::microkernel_process::types::WitProtomessageType;
use bindings::component::microkernel_process::types::WitRequestTypeWithTarget;

struct Component;

fn parse_command(line: String) {
    let (head, tail) = line.split_once(" ").unwrap_or((&line, ""));
    match head {
        "!message" => {
            let (target_server, tail) = match tail.split_once(" ") {
                Some((s, t)) => (s, t),
                None => {
                    bindings::print_to_terminal("invalid command");
                    return;
                }
            };
            let (target_app, payload) = match tail.split_once(" ") {
                Some((a, p)) => (a, p),
                None => {
                    bindings::print_to_terminal("invalid command");
                    return;
                }
            };

            bindings::yield_results(vec![
                (
                    WitProtomessage {
                        protomessage_type: WitProtomessageType::Request(WitRequestTypeWithTarget {
                            is_expecting_response: false,
                            target_ship: target_server,
                            target_app: target_app,
                        }),
                        payload: &WitPayload {
                            json: Some(payload.into()),
                            bytes: None,
                        },
                    },
                    "",
                ),
            ].as_slice());
        }
        _ => {
            bindings::print_to_terminal("invalid command");
            return
        }
    }
}

impl bindings::MicrokernelProcess for Component {
    fn run_process(our_name: String, process_name: String) {
        assert_eq!(process_name, "terminal");
        bindings::print_to_terminal(format!("{} terminal: running", our_name.clone()).as_str());

        loop {
            let (message, _) = bindings::await_next_message();
            let stringy = message.payload.bytes.unwrap_or(vec![]);
            parse_command(String::from_utf8(stringy).unwrap_or("".into())); // gross
        }
    }
}

bindings::export!(Component);
