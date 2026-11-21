//! `lisage-debug` — interactive debugger over a trace (D3-1).
//!
//! Runs the standard scenario (saturation n=4, DoD) and reads commands
//! from stdin: step / step-over / step-out / next-op / next-round /
//! back / goto / until / where / frame / continue, quit.

use std::io::{BufRead, Write};

use lisage_liseuse::debugger::{Debugger, parse_command, standard_scenario};

fn main() {
    let events = standard_scenario(4);
    let rounds = lisage_liseuse::debugger::round_numbers(&events).len();
    let mut debugger = Debugger::new(events);
    println!(
        "lisage-debug: standard scenario n=4, {} events, {rounds} rounds",
        debugger.length()
    );
    println!(
        "commands: step back goto <n> step-over step-out next-op next-round until <kind> where frame continue quit"
    );
    println!("{}", debugger.run(lisage_liseuse::debugger::Command::Where));
    let stdin = std::io::stdin();
    loop {
        print!("(lisage-debug) ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if matches!(line, "quit" | "exit" | "q") {
            break;
        }
        match parse_command(line) {
            Some(command) => println!("{}", debugger.run(command)),
            None => println!("unknown command: {line}"),
        }
    }
}
