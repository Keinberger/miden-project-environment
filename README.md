# Miden Project

The Miden workspace structure.

## **Structure**

```
miden-project/
├── contracts/                   # Each contract as individual crate
│   ├── counter-account/
│   └── increment-note/
├── scripts/                     # Script crate for on-chain interactions
├── tests/                       # Tests crate
├── helpers/                     # Temporary helper crate (will be removed in future)
├── Cargo.toml                   # Workspace root
└── rust-toolchain.toml          # Temporary Rust toolchain specification (will be removed in the future)
```

> **Note**: The `helpers/` directory and `rust-toolchain.toml` file are temporary and exist only to make working with the Rust compiler easier. They will be removed in the future.

## **Design Philosophy**

This workspace structure provides a baseline for building complex applications. Both `scripts/` and `tests/` are organized as their own separate crates to accommodate the needs of growing applications.

As applications increase in complexity, they often require:

- Custom dependencies and version requirements specific to scripts or tests
- Sophisticated tooling and utilities
- Independent configuration and build settings
- Clear separation of concerns

By structuring scripts and tests as separate crates from the start, this workspace provides the flexibility to scale with your application's needs.

Furthermore, in the future both the tests and scripts will be natively integrated into the `midenup` CLI tooling, which will elevate the development experience.

## **Scripts: On-Chain Interactions**

Code for making on-chain interactions with the contracts are placed as Rust binaries inside of the `scripts/` crate. Each binary represents a specific interaction or script that can be executed against the contracts.

## **Tests**

Tests are organized in the `tests/` crate, with test files located in the `tests/tests/` directory.

## **Commands**

```bash
# Run any script binary
cd scripts && cargo run --bin increment_count

# Run tests
cd tests && cargo test                      # Run all tests
cd tests && cargo test counter_test         # Run specific test file
```
