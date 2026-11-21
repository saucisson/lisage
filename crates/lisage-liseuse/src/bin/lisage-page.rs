//! D6-1 — `lisage-page`: generate a self-contained offline HTML page
//! embedding the trace, index and navigation controls.
//!
//! Usage: `lisage-page [n] [output.html]`
//!   n          scenario size (default 4, the philosophers saturation)
//!   output.html  output file (default: stdout)

use std::io::Write;

use lisage_liseuse::page::standard_offline_page;

fn main() {
    let n: u64 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "4".to_string())
        .parse()
        .expect("n is an integer");
    let output = std::env::args().nth(2);
    let html = standard_offline_page(n);
    match output {
        Some(path) => {
            let mut file = std::fs::File::create(&path).expect("creating output file");
            file.write_all(html.as_bytes()).expect("writing HTML");
            eprintln!("page written to {path} ({} bytes)", html.len());
        }
        None => println!("{html}"),
    }
}
