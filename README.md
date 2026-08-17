# refDB

`refDB` is an educational database project focused on building a small but sophisticated storage engine around a classic Log-Structured Merge Tree (LSM) index.

## Goals

- Learn how modern storage engines are structured
- Implement a minimal key-value database from first principles
- Use an LSM-based design with in-memory writes and immutable on-disk tables
- Keep the codebase readable, modular, and suitable for experimentation

## Core Ideas

- **Memtable**: stores recent writes in memory
- **Write-Ahead Log (WAL)**: preserves durability before data reaches disk
- **SSTables**: immutable sorted files used for persistent storage
- **Compaction**: merges SSTables to reduce read amplification and reclaim space
- **Indexing**: enables efficient lookup over sorted data

## Status

This project is in early development and is intended for educational use.

## Running

```bash
cargo run
```

## Testing

```bash
cargo test
```

## License

TBD
