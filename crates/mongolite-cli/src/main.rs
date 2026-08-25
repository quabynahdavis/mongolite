use std::env;
use std::io::Write;

use mongolite::db::Database;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: mongolite-cli <database-path>");
        std::process::exit(1);
    }

    let path = &args[1];
    let mut db = match Database::create(path) {
        Ok(db) => db,
        Err(_) => Database::open(path).expect("Failed to open database"),
    };

    println!("MongoLite CLI v0.1.0");
    println!("Connected to: {}", path);
    println!("Commands: tables, insert, find, count, delete, stats, quit");
    println!();

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    loop {
        print!("mongolite> ");
        stdout.flush().unwrap();

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input == "quit" || input == "exit" {
            break;
        }

        if let Err(e) = execute(&mut db, input) {
            eprintln!("Error: {}", e);
        }
    }
}

fn execute(db: &mut Database, input: &str) -> Result<(), mongolite::error::Error> {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let command = parts[0].to_lowercase();
    let args = parts.get(1).unwrap_or(&"");

    match command.as_str() {
        "tables" | "show collections" => {
            let collections = db.list_collections()?;
            if collections.is_empty() {
                println!("No collections.");
            } else {
                for name in collections {
                    println!("  {}", name);
                }
            }
        }
        "insert" => {
            let parts: Vec<&str> = args.splitn(2, ' ').collect();
            if parts.len() < 2 {
                println!("Usage: insert <collection> <json>");
                return Ok(());
            }
            let doc: bson::Document = serde_json::from_str(parts[1])
                .map_err(|e| mongolite::error::Error::InvalidQuery(format!("invalid JSON: {}", e)))?;
            let mut coll = db.collection(parts[0]);
            let result = coll.insert_one(doc)?;
            println!("Inserted _id: {}", result.inserted_id);
        }
        "find" => {
            let parts: Vec<&str> = args.splitn(2, ' ').collect();
            if parts.is_empty() || parts[0].is_empty() {
                println!("Usage: find <collection> [filter]");
                return Ok(());
            }
            let filter = if parts.len() > 1 {
                Some(serde_json::from_str::<bson::Document>(parts[1]).map_err(|e| {
                    mongolite::error::Error::InvalidQuery(format!("invalid filter JSON: {}", e))
                })?)
            } else {
                None
            };
            let coll = db.collection(parts[0]);
            let docs = coll.find(filter)?;
            if docs.is_empty() {
                println!("No documents found.");
            } else {
                for doc in &docs {
                    println!("{}", serde_json::to_string_pretty(doc).unwrap_or_default());
                }
                println!("{} document(s) found.", docs.len());
            }
        }
        "count" => {
            let parts: Vec<&str> = args.splitn(2, ' ').collect();
            if parts.is_empty() || parts[0].is_empty() {
                println!("Usage: count <collection> [filter]");
                return Ok(());
            }
            let filter = if parts.len() > 1 {
                Some(serde_json::from_str::<bson::Document>(parts[1]).map_err(|e| {
                    mongolite::error::Error::InvalidQuery(format!("invalid filter JSON: {}", e))
                })?)
            } else {
                None
            };
            let coll = db.collection(parts[0]);
            let count = coll.count(filter)?;
            println!("Count: {}", count);
        }
        "delete" => {
            let parts: Vec<&str> = args.splitn(2, ' ').collect();
            if parts.len() < 2 {
                println!("Usage: delete <collection> <filter>");
                return Ok(());
            }
            let filter: bson::Document = serde_json::from_str(parts[1])
                .map_err(|e| mongolite::error::Error::InvalidQuery(format!("invalid filter JSON: {}", e)))?;
            let mut coll = db.collection(parts[0]);
            let result = coll.delete_many(filter)?;
            println!("Deleted {} document(s).", result.deleted_count);
        }
        "stats" => {
            let collections = db.list_collections()?;
            println!("Database statistics:");
            println!("  Collections: {}", collections.len());
            for name in &collections {
                let coll = db.collection(name);
                let count = coll.count(None)?;
                println!("  {}: {} documents", name, count);
            }
        }
        "help" => {
            println!("Commands:");
            println!("  tables                   List all collections");
            println!("  insert <coll> <json>    Insert a document");
            println!("  find <coll> [filter]    Find documents");
            println!("  count <coll> [filter]   Count documents");
            println!("  delete <coll> <filter>  Delete documents");
            println!("  stats                   Show statistics");
            println!("  quit                    Exit");
        }
        _ => println!("Unknown command: '{}'. Type 'help' for commands.", command),
    }

    Ok(())
}
