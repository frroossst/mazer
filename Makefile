.PHONY: *


clean:
	cargo clean -p mazer-atog
	cargo clean -p mazer-cli
	cargo clean -p mazer-dbg
	cargo clean -p mazer-html
	cargo clean -p mazer-lisp
	cargo clean -p mazer-parser
	cargo clean -p mazer-render
	cargo clean -p mazer-stdlib
	cargo clean -p mazer-types

install:
	cargo install --force --path ./mazer-cli/

build:
	cargo build

release:
	cargo build --release


