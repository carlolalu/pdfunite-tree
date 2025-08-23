use pdfunite_tree::run;

fn main() {
    // following minigrep from the official Rust book
    if let Err(err) = run() {
        eprintln!("Application error: {}", err);
        std::process::exit(1);
    }
}
