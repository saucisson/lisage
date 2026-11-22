//! D6-3 -- `lisage-dap`: DAP (Debug Adapter Protocol) server for the
//! lisage trace debugger. Runs on stdio. `--port N` is accepted but not
//! yet implemented: the server warns on stderr and falls back to stdio.
//!
//! Usage: `lisage-dap [--port N]`
//!
//! DoD: VS Code -- breakpoint " round 12 " operational on philosophers.

use lisage_liseuse::dap::run_stdio;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--port") {
        eprintln!(
            "lisage-dap: TCP socket mode not yet implemented in this increment; falling back to stdio"
        );
    }
    run_stdio();
}
