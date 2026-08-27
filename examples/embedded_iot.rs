//! Embedded IoT use case: a small, single-file database for local sensor data.
//!
//! This example shows how to use MongoLite as an embedded store for an
//! IoT device that needs to persist sensor readings locally without a
//! running database server.

use mongolite::Database;
use bson::doc;

fn main() -> Result<(), mongolite::error::Error> {
    // Use a path appropriate for the device (e.g. on-device flash).
    let path = "/tmp/sensor_data.mongolite";
    let mut db = Database::create(path)?;

    // Sensor readings are stored in a "readings" collection.
    let mut readings = db.collection("readings");

    // Simulate 10 sensor readings.
    for i in 0..10 {
        let reading = doc! {
            "sensor_id": format!("sensor-{:03}", i % 3),
            "temperature": 20.0 + (i as f64) * 0.5,
            "humidity": 40.0 + (i as f64) * 1.5,
            "timestamp": bson::DateTime::now().to_chrono(),
        };
        readings.insert_one(reading)?;
    }

    // Query: find all readings from sensor-001 with temperature > 22.0
    let filter = doc! {
        "sensor_id": "sensor-001",
        "temperature": { "$gt": 22.0 }
    };

    let results = readings.find(Some(filter))?;
    println!("Found {} hot readings from sensor-001", results.len());
    for doc in &results {
        println!(
            "  temp={:?} humidity={:?}",
            doc.get_f64("temperature").unwrap(),
            doc.get_f64("humidity").unwrap()
        );
    }

    // Persist to disk.
    db.flush()?;
    println!("Sensor data persisted to {}", path);

    Ok(())
}