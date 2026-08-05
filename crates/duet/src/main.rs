//! Duet Orthodox File Manager Application Entry Point (Task T-11.1.5).

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut left_path = None;
    let mut right_path = None;

    let mut iter = args.into_iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-v" | "--version" => {
                println!("Duet File Manager v0.1.0-alpha");
                println!("GPU-accelerated Orthodox File Manager for Linux (Rust + GPUI)");
                return;
            }
            "-h" | "--help" => {
                println!("Usage: duet [OPTIONS] [PATH]");
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
            "--left" => {
                if let Some(p) = iter.next() {
                    left_path = Some(p);
                }
            }
            "--right" => {
                if let Some(p) = iter.next() {
                    right_path = Some(p);
                }
            }
            "--goto" | "--new-tab" => {
                if let Some(p) = iter.next() {
                    left_path = Some(p);
                }
            }
            p => {
                if !p.starts_with('-') {
                    left_path = Some(p.to_string());
                }
            }
        }
    }

    println!("Launching Duet Orthodox File Manager...");
    duet_ui::run_app_with_paths(left_path, right_path);
}
