//! Duet Orthodox File Manager Application Entry Point (Task T-11.1.5).

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    for arg in &args[1..] {
        match arg.as_str() {
            "-v" | "--version" => {
                println!("Duet File Manager v0.1.0-alpha");
                println!("GPU-accelerated Orthodox File Manager for Linux (Rust + GPUI)");
                return;
            }
            "-h" | "--help" => {
                println!("Usage: duet [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --left <PATH>       Set left panel starting directory");
                println!("  --right <PATH>      Set right panel starting directory");
                println!("  --new-tab <PATH>    Open path in a new tab");
                println!("  --goto <PATH>       Navigate active panel to path");
                println!("  -v, --version       Display version information");
                println!("  -h, --help          Display help usage");
                return;
            }
            _ => {}
        }
    }

    println!("Launching Duet Orthodox File Manager...");
    duet_ui::run_app();
}
