//! `lisage-debug` — interactive debugger over a trace (D3-1 + D3-3).
//!
//! Runs the standard scenario (saturation n=4, DoD) and reads commands
//! from stdin: navigation (step / step-over / step-out / next-op /
//! next-round / back / goto / until / where / frame / continue),
//! inspection (node <id> / memo / memoire / diff / eval <valuation> /
//! display / ops), svg (current diagram rendering), quit.

use std::io::{BufRead, Write};

use lisage_liseuse::debugger::{Debugger, parse_command, round_numbers};
use lisage_liseuse::inspection::{render_current_svg, standard_scenario_full};

fn main() {
    let (events, pool, root) = standard_scenario_full(4);
    let rounds = round_numbers(&events).len();
    let events_for_svg = events.clone();
    let mut debugger = Debugger::new(events);
    println!(
        "lisage-debug: standard scenario n=4, {} events, {rounds} rounds",
        debugger.length()
    );
    println!(
        "navigation: step back goto <n> step-over step-out next-op next-round until <kind> where frame continue"
    );
    println!("inspection: node <id> memo memoire diff eval <valuation> display ops svg");
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
        if line == "svg" {
            println!(
                "{}",
                render_current_svg(&pool, root, &events_for_svg, debugger.position())
            );
            continue;
        }
        match parse_command(line) {
            Some(command) => println!("{}", debugger.run(command)),
            None => println!("unknown command: {line}"),
        }
    }
}
