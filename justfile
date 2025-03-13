export MAZER_TEMPLATE_DIR := "/home/home/Desktop/Projects/mazer/templates/"

# run with hello.zr argument
run:
    cargo run examples/hello.zr

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

# publish to crates.io
publish:
    cargo publish mazer-cli


# run cargo fmt
format:
    cargo 

# run cargo clippy
clippy:
    cargo clippy
