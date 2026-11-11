.PHONY: *

build:
	cargo build

clean:
	cargo clean -p mazer-syntax
	cargo clean -p mazer-math
	cargo clean -p mazer-doc
	cargo clean -p mazer-cli
	cargo clean -p mazer-wasm
	cargo clean -p mazer-lsp
	cargo clean -p mazer-dbg

check:
	cargo test
	cargo clippy --all-targets

install:
	cargo install --force --path ./mazer-cli/

release:
	cargo build --release

# The CLI with LaTeX output (`mazer --target latex`).
latex:
	cargo build --release -p mazer-cli --features latex

# Features the Rust toolchain emits that wasm-opt (v105) must be told to allow.
WASM_OPT_FEATURES := --enable-bulk-memory --enable-sign-ext \
	--enable-nontrapping-float-to-int --enable-mutable-globals \
	--enable-multivalue --enable-reference-types

wasm:
	wasm-pack build mazer-wasm --target web --out-dir pkg
	wasm-opt -Oz --converge $(WASM_OPT_FEATURES) \
		-o mazer-wasm/pkg/mazer_wasm_bg.wasm mazer-wasm/pkg/mazer_wasm_bg.wasm

deploy: wasm ## copies to adhyan.ca
	cp ./mazer-wasm/pkg/mazer_wasm_bg.wasm /home/home/Desktop/Projects/adhyan.ca/
	cp ./mazer-wasm/pkg/mazer_wasm.js /home/home/Desktop/Projects/adhyan.ca/



