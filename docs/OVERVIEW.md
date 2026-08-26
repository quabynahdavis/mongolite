# MongoLite Overview

MongoLite is a lightweight, single‑file, server‑less document database that implements a subset of the MongoDB API. It is written in Rust and provides:

- A core library (`mongolite`) exposing `Database`, `Collection`, cursors, and query helpers.
- An interactive CLI (`mongolite-cli`) for quick manual operations.
- A C‑FFI wrapper (`mongolite-ffi`) enabling use from other languages.

The database stores data in a custom B‑Tree backed storage engine with write‑ahead logging (WAL) for durability. It is intended for embedded scenarios, testing, and small‑scale applications where a full MongoDB server would be overkill.

## Features
- BSON‑compatible document model.
- Basic CRUD operations (`insert`, `find`, `count`, `delete`).
- Collection listing and per‑collection statistics.
- Simple query language based on JSON filters.
- File‑based persistence with automatic recovery from crashes.
- FFI bindings for C and other languages.

## Project Structure
- `crates/mongolite/` – core library.
- `crates/mongolite-cli/` – command‑line interface binary.
- `crates/mongolite-ffi/` – C‑FFI wrapper.
- `docs/` – documentation (this folder).
