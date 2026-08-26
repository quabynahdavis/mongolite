use std::time::Instant;

use bson::doc;
use tempfile::TempDir;

use mongolite::db::Database;

fn main() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bench.mongolite");
    let mut db = Database::create(&path).unwrap();

    println!("MongoLite Benchmarks");
    println!("==================\n");

    let start = Instant::now();
    let mut coll = db.collection("bench");
    for i in 0..1000 {
        coll.insert_one(doc! {
            "index": i,
            "name": format!("user_{}", i),
            "active": i % 2 == 0,
        })
        .unwrap();
    }
    let elapsed = start.elapsed();
    println!("Insert 1000 docs: {:?}", elapsed);

    let start = Instant::now();
    let count = coll.count(None).unwrap();
    let elapsed = start.elapsed();
    println!("Count {} docs: {:?}", count, elapsed);

    let start = Instant::now();
    let docs = coll.find(Some(doc! { "active": true })).unwrap();
    let elapsed = start.elapsed();
    println!("Find {} active docs: {:?}", docs.len(), elapsed);

    let start = Instant::now();
    let docs = coll.find(Some(doc! { "index": { "$gte": 500 } })).unwrap();
    let elapsed = start.elapsed();
    println!("Find {} docs with index >= 500: {:?}", docs.len(), elapsed);

    let start = Instant::now();
    let docs = coll.find(Some(doc! {
        "$or": [
            { "index": { "$lt": 100 } },
            { "index": { "$gte": 900 } }
        ]
    }))
    .unwrap();
    let elapsed = start.elapsed();
    println!("Find {} docs with $or: {:?}", docs.len(), elapsed);

    let start = Instant::now();
    let docs = coll.find_with_options(
        None,
        Some(doc! { "index": -1 }),
        Some(100),
        Some(10),
        Some(doc! { "name": 1, "_id": 0 }),
    )
    .unwrap();
    let elapsed = start.elapsed();
    println!("Find with sort/skip/limit/projection: {} docs in {:?}", docs.len(), elapsed);

    println!("\nDone!");
}
