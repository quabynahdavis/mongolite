//! Test Fixture use case: quickly spin up an isolated database for testing.
//!
//! This example demonstrates how to use MongoLite as a temporary test database
//! that is automatically cleaned up when the test finishes.

use mongolite::Database;
use bson::doc;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory that will be cleaned up on drop.
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.mongolite");

    // Open a new database in the temp directory.
    let mut db = Database::create(&db_path)?;

    // Set up test data: a "users" collection with some sample documents.
    let mut users = db.collection("users");
    users.insert_one(doc! { "name": "Alice", "role": "admin" })?;
    users.insert_one(doc! { "name": "Bob", "role": "user" })?;
    users.insert_one(doc! { "name": "Charlie", "role": "user" })?;

    // Run test queries.
    let admins = users.find(doc! { "role": "admin" })?;
    assert_eq!(admins.len(), 1, "Expected exactly one admin");
    println!("Admin user: {:?}", admins[0].get_str("name").unwrap());

    let user_count = users.count(doc! { "role": "user" })?;
    assert_eq!(user_count, 2, "Expected two users");
    println!("User count: {}", user_count);

    // No manual cleanup needed — TempDir drops here and deletes everything.
    println!("Test passed! Temporary database at {} cleaned up automatically.", db_path.display());
    Ok(())
}