//! MEM-19 runner: layout on/off confrontation. Runs the deterministic
//! seeded BDD workload under one layout (`on` = inline exact-class
//! entries, MEM-16; `off` = catch-all boxed entries, pre-MEM-16) and
//! prints one JSON line with the structural counters, the time and the
//! peak RSS. The node count is identical either way (C1): only the
//! container differs.
//!
//! Usage: `layout <on|off> <n>`

use std::time::Instant;

use lisage_algebra::interface::{Anchor, ArcRun, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::bdd::BddController;
use lisage_core::observer::NoOpObserver;

/// Fixed seed of the randomized IDs (same as the MEM-18 thresholds).
const SEED_IDS: u64 = 0x9E37_79B9_7F4A_7C15;

/// xorshift64* — reproducible randomized arc keys.
struct IdsAleatoires {
    state: u64,
}

impl IdsAleatoires {
    fn new(seed: u64) -> Self {
        IdsAleatoires { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// Peak process RSS in kibibytes (`VmHWM` of `/proc/self/status`).
fn rss_max_ko() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("reading /proc/self/status");
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|reste| reste.trim().strip_suffix("kB"))
        .and_then(|nombre| nombre.trim().parse::<u64>().ok())
        .expect("VmHWM present")
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let (layout, n) = match arguments.as_slice() {
        [layout, n] => (layout.as_str(), n.parse::<u32>().expect("readable n")),
        _ => {
            eprintln!("usage: layout <on|off> <n>");
            std::process::exit(2);
        }
    };
    let inline = match layout {
        "on" => true,
        "off" => false,
        autre => {
            eprintln!("layout must be on or off, got: {autre}");
            std::process::exit(2);
        }
    };
    let mut pool = Arena::new(BddController::new(64), Box::new(NoOpObserver));
    pool.set_layout_inline(inline);
    let t0 = pool.terminal(Anchor(0), false);
    let t1 = pool.terminal(Anchor(0), true);
    let mut ids = IdsAleatoires::new(SEED_IDS);
    let debut = Instant::now();
    for i in 0..n {
        let k0 = ids.next_u64() % 2;
        pool.intern(
            Anchor(0),
            Form::Node {
                level: Level(i),
                arcs: ArcRun::from_pairs([(k0, t0), (1 - k0, t1)]),
            },
        );
    }
    let temps_ms = debut.elapsed().as_secs_f64() * 1000.0;
    let nodes = pool.live() as u64;
    let bytes_par_noeud = pool.bytes_per_node();
    let octets_arenas = pool.arenas_bytes() as f64;
    let rss = rss_max_ko();
    // One JSON line, stable field order.
    println!(
        "{{\"layout\":\"{layout}\",\"n\":{n},\"nodes\":{nodes},\"bytes_par_noeud\":{bytes_par_noeud:.2},\"octets_arenas_par_noeud\":{:.2},\"temps_ms\":{temps_ms:.3},\"rss_max_ko\":{rss}}}",
        octets_arenas / nodes as f64
    );
    let _ = NodeId(0);
}
