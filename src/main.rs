use std::{env, process};

mod runner;
use runner::Config;

fn main() {
    // Handle `--help` and `-h`
    if env::args().any(|arg| arg == "--help" || arg == "-h") {
        Config::print_config();
        process::exit(0);
    }

    // Build config
    let config = match Config::build(env::args()) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("\x1B[01;31mConfiguration error\x1B[00m: {}", error);
            Config::print_config();
            process::exit(1);
        }
    };

    // Run the program
    if let Err(error) = runner::run(config) {
        eprintln!("\x1B[01;31mApplication error\x1B[00m: {}", error);
        process::exit(2);
    }
}
