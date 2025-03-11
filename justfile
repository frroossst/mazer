

# run cargo fmt
format:
    cargo 

# run cargo clippy
clippy:
    cargo clippy

# run all tests
tests: core-tests

# run mazer-core tests
core-tests:
    cd mazer-core && cargo exam

