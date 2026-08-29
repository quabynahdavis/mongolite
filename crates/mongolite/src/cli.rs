use std::io::{self, Write};

use bson::Document;

use crate::db::Database;
use crate::error::Result;

pub struct Cli {
    db: Database,
}

impl Cli {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn run(&mut self) -> Result<()> {
        println!("MongoLite CLI v0.1.0");
        println!("Type 'help' for commands, 'quit' to exit.");
        println!();

        let stdin = io::stdin();
        let mut stdout = io::stdout();

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

            if let Err(e) = self.execute(input) {
                eprintln!("Error: {}", e);
            }
        }

        Ok(())
    }

    fn execute(&mut self, input: &str) -> Result<()> {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let command = parts[0].to_lowercase();
        let args = parts.get(1).unwrap_or(&"");

        match command.as_str() {
            "quit" | "exit" => std::process::exit(0),
            "help" => self.print_help(),
            "tables" | "show collections" => self.list_collections()?,
            "insert" => self.insert(args)?,
            "find" => self.find(args)?,
            "count" => self.count(args)?,
            "delete" => self.delete(args)?,
            "stats" => self.stats()?,
            _ => println!("Unknown command: '{}'. Type 'help' for commands.", command),
        }

        Ok(())
    }

    fn print_help(&self) {
        println!("Commands:");
        println!("  tables                   List all collections");
        println!("  insert <coll> <json>    Insert a document into a collection");
        println!("  find <coll> [filter]    Find documents in a collection");
        println!("  count <coll> [filter]   Count documents in a collection");
        println!("  delete <coll> <filter>  Delete documents by _id");
        println!("  stats                   Show database statistics");
        println!("  help                    Show this help");
        println!("  quit                    Exit the CLI");
    }

    fn list_collections(&self) -> Result<()> {
        let collections = self.db.list_collections()?;
        if collections.is_empty() {
            println!("No collections.");
        } else {
            for name in collections {
                println!("  {}", name);
            }
        }
        Ok(())
    }

    fn insert(&mut self, args: &str) -> Result<()> {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.len() < 2 {
            println!("Usage: insert <collection> <json>");
            return Ok(());
        }

        let coll_name = parts[0];
        let json_str = parts[1];

        let doc: Document = serde_json::from_str(json_str)
            .map_err(|e| crate::error::Error::InvalidQuery(format!("invalid JSON: {}", e)))?;

        let mut coll = self.db.collection(coll_name);
        let result = coll.insert_one(doc)?;
        println!("Inserted _id: {}", result.inserted_id);
        Ok(())
    }

    fn find(&mut self, args: &str) -> Result<()> {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.is_empty() || parts[0].is_empty() {
            println!("Usage: find <collection> [filter]");
            return Ok(());
        }

        let coll_name = parts[0];
        let filter = if parts.len() > 1 {
            Some(serde_json::from_str::<Document>(parts[1]).map_err(|e| {
                crate::error::Error::InvalidQuery(format!("invalid filter JSON: {}", e))
            })?)
        } else {
            None
        };

        let coll = self.db.collection(coll_name);
        let docs = coll.find(filter)?;

        if docs.is_empty() {
            println!("No documents found.");
        } else {
            for doc in &docs {
                println!("{}", serde_json::to_string_pretty(doc).unwrap_or_default());
            }
            println!("{} document(s) found.", docs.len());
        }
        Ok(())
    }

    fn count(&mut self, args: &str) -> Result<()> {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.is_empty() || parts[0].is_empty() {
            println!("Usage: count <collection> [filter]");
            return Ok(());
        }

        let coll_name = parts[0];
        let filter = if parts.len() > 1 {
            Some(serde_json::from_str::<Document>(parts[1]).map_err(|e| {
                crate::error::Error::InvalidQuery(format!("invalid filter JSON: {}", e))
            })?)
        } else {
            None
        };

        let coll = self.db.collection(coll_name);
        let count = coll.count(filter)?;
        println!("Count: {}", count);
        Ok(())
    }

    fn delete(&mut self, args: &str) -> Result<()> {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.len() < 2 {
            println!("Usage: delete <collection> <filter>");
            return Ok(());
        }

        let coll_name = parts[0];
        let filter: Document = serde_json::from_str(parts[1]).map_err(|e| {
            crate::error::Error::InvalidQuery(format!("invalid filter JSON: {}", e))
        })?;

        let mut coll = self.db.collection(coll_name);
        let result = coll.delete_many(filter)?;
        println!("Deleted {} document(s).", result.deleted_count);
        Ok(())
    }

    fn stats(&mut self) -> Result<()> {
        let collections = self.db.list_collections()?;
        println!("Database statistics:");
        println!("  Collections: {}", collections.len());
        for name in &collections {
            let coll = self.db.collection(name);
            let count = coll.count(None)?;
            println!("  {}: {} documents", name, count);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_cli() -> (Cli, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");
        let db = Database::create(&path).unwrap();
        (Cli::new(db), dir)
    }

    #[test]
    fn test_cli_insert_and_find() {
        let (mut cli, _dir) = create_test_cli();

        cli.execute(r#"insert users {"name": "Alice", "age": 30}"#)
            .unwrap();
        cli.execute(r#"insert users {"name": "Bob", "age": 25}"#)
            .unwrap();

        let coll = cli.db.collection("users");
        let docs = coll.find(None).unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_cli_count() {
        let (mut cli, _dir) = create_test_cli();

        cli.execute(r#"insert users {"name": "Alice"}"#).unwrap();
        cli.execute(r#"insert users {"name": "Bob"}"#).unwrap();

        let coll = cli.db.collection("users");
        assert_eq!(coll.count(None).unwrap(), 2);
    }

    #[test]
    fn test_cli_delete() {
        let (mut cli, _dir) = create_test_cli();

        cli.execute(r#"insert users {"name": "Alice"}"#).unwrap();

        let coll = cli.db.collection("users");
        assert_eq!(coll.count(None).unwrap(), 1);
    }

    #[test]
    fn test_cli_list_collections() {
        let (mut cli, _dir) = create_test_cli();

        cli.execute(r#"insert users {"name": "Alice"}"#).unwrap();
        cli.execute(r#"insert posts {"title": "Hello"}"#).unwrap();

        let collections = cli.db.list_collections().unwrap();
        assert!(collections.contains(&"users".to_string()));
        assert!(collections.contains(&"posts".to_string()));
    }
}
