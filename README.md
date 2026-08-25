# MongoLite

A serverless, single-file, MongoDB-compatible document database written in Rust.

## Status

🚧 Early development — v0.1.0 in progress.

## Architecture

- **Single-file storage** — entire database in one `.mongolite` file
- **BSON document format** — MongoDB-compatible encoding
- **B+tree indexes** — efficient document retrieval
- **WAL crash safety** — write-ahead log for durability
- **Memory-mapped I/O** — high-performance reads/writes

## Project Structure

```
mongolite/
├── crates/
│   ├── mongolite/        # Core library
│   ├── mongolite-cli/    # CLI tool
│   └── mongolite-ffi/    # C bindings
├── tests/                # Integration tests
├── benches/              # Benchmarks
└── docs/                 # Documentation
```

## License

Apache-2.0
