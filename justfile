
# run all tests
tests: core-tests

# run mazer-core tests
core-tests:
    cd mazer-core && cargo exam

# just build
build:
    cargo build

# build for release
release:
    cargo build --release

# run cargo fmt
format:
    cargo 

# run cargo clippy
clippy:
    cargo clippy
