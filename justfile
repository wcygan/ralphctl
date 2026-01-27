# Development tasks for ralphctl

# Default: run all checks
default: check

# Format code
fmt:
    cargo fmt

# Run clippy lints
lint:
    cargo clippy -- -D warnings

# Auto-fix clippy warnings
fix:
    cargo clippy --fix --allow-dirty --allow-staged

# Check formatting + lints (CI parity)
check:
    cargo fmt --check
    cargo clippy -- -D warnings

# Run all tests
test:
    cargo test

# Build debug binary
build:
    cargo build

# Build release binary
release:
    cargo build --release

# Run full CI suite locally
ci: check test

# Format, lint, test, then build
all: fmt lint test build
