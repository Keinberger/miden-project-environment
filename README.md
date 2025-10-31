# Miden Project

The Miden workspace structure.

## **Structure**

```
miden-project/
├── contracts/                   # Each contract as individual workspace member crate
│   ├── counter-account/
│   └── increment-note/
├── src/                         # Root package source
│   ├── bin/                     # Rust binaries for on-chain interactions
│   │   └── increment_count.rs
│   ├── helpers.rs               # Temporary helper module (will be removed in future)
│   └── lib.rs
├── tests/                       # Tests
├── Cargo.toml
└── rust-toolchain.toml          # Temporary Rust toolchain specification (will be removed in the future)
```

> **Note**: The `helpers.rs` module and `rust-toolchain.toml` file are temporary and exist only to make working with the Rust compiler easier. They will be removed in the future.

## **Project Structure**

This project uses a hybrid package-workspace structure where:

- The root is both a Rust package and a workspace
- Contracts are organized as separate workspace member crates
- Scripts for on-chain interactions are Rust binaries in `src/bin/`
- Tests are integration tests in the `tests/` directory

## **Scripts: On-Chain Interactions**

Code for making on-chain interactions with the contracts are placed as Rust binaries inside of `src/bin/`. Each binary represents a specific interaction or script that can be executed against the contracts.

## **Tests**

Tests are organized as integration tests in the `tests/` directory, which test the root package and its binaries.

## **Commands**

```bash
# Run any script binary
cargo run --bin increment_count

# Run tests
cargo test                      # Run all tests
cargo test counter_test         # Run specific test
```
