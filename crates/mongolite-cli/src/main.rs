use std::env;
use std::path::PathBuf;

use mongolite::cli::Cli;
use mongolite::db::Database;

fn main() {
    let args: Vec<String> = env::args().collect();

    let path = match args.len() {
        1 => {
            eprintln!("Usage: mongolite-cli <database-path>");
            std::process::exit(1);
        }
        _ => PathBuf::from(&args[1]),
    };

    let mut db = match Database::create(&path) {
        Ok(db) => db,
        Err(_) => Database::open(&path).expect("Failed to open database"),
    };

    println!("MongoLite CLI v0.1.0");
    println!("Connected to: {}", path.display());
    println!("Commands: tables, insert, find, count, delete, stats, quit");
    println!();

    let mut cli = Cli::new(db);
    if let Err(e) = cli.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}