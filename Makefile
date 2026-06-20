.PHONY: *

build:
	cargo build

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

release:
	cargo build --release

wasm:
	wasm-pack build mazer-wasm --target web --out-dir pkg

deploy: wasm ## copies to adhyan.ca
	cp ./mazer-wasm/pkg/mazer_wasm_bg.wasm /home/home/Desktop/Projects/adhyan.ca/
	cp ./mazer-wasm/pkg/mazer_wasm.js /home/home/Desktop/Projects/adhyan.ca/



