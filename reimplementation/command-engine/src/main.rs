use altron_command_engine::{Decision, Router};

fn main() {
    let request = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    match Router::desktop_defaults().resolve(&request) {
        Decision::Ready(action) => println!("Action plan: {action:?}"),
        Decision::Clarify(actions) => println!("Clarification required: {actions:?}"),
        Decision::Unknown => {
            eprintln!("Command not recognized; no action executed");
            std::process::exit(2);
        }
    }
}
