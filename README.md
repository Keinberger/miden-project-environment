# Miden Project

This is the Miden workspace structure.

## **Structure**

```
miden-project/
├── contracts/                   # Each contract as individual crate
│   ├── counter-account/
│   │   ├── Cargo.toml
│   │   └── counter.rs
│   ├── increment-note/
│   │   ├── Cargo.toml
│   │   └── increment_count.rs
│   └── target/packages/         # Compiled .masp files
├── scripts/                     # Script binaries
│   ├── deploy.rs
│   ├── increment_count.rs
│   └── query_counter.rs
├── tests/
│   ├── counter_test.rs
│   ├── integration_test.rs
│   └── e2e_test.rs
├── helpers/                     # Single file helper crate
│   ├── Cargo.toml
│   └── helpers.rs
├── Cargo.toml                   # Workspace root as package + workspace
└── miden-project.toml
```

## **Commands**

```bash
# Run any script
cargo run --bin increment_count

# Run tests
cargo test                      # All tests
cargo test counter_test         # Specific test file
cargo test integration_test     # Specific test file
```
