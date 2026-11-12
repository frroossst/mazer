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

# The wasm module and its glue in mazer-wasm/pkg/, plus a copy of the equation
# search script (assets/mazer-search.js), which mazer.js loads after rendering.
wasm:
	wasm-pack build mazer-wasm --target web --out-dir pkg
	wasm-opt -Oz --converge $(WASM_OPT_FEATURES) \
		-o mazer-wasm/pkg/mazer_wasm_bg.wasm mazer-wasm/pkg/mazer_wasm_bg.wasm
	cp assets/mazer-search.js mazer-wasm/pkg/mazer_search.js

# Where the site lives, and what `make deploy` is allowed to touch there.
SITE ?= /home/home/Desktop/Projects/adhyan.ca
SITE_NOTES ?= $(SITE)/notes
# The native CLI used to vet the notes; build it first (`cargo build --release`).
MAZER ?= cargo run --quiet --release -p mazer-cli --

# Runs `mazer --check --strict` on every note under SITE_NOTES and stops, with
# the list of failing files, if any fails (or if the directory is missing).
# Reads the notes; never writes them.
check-site-notes:
	@[ -d "$(SITE_NOTES)" ] || { echo "deploy aborted: no notes directory at $(SITE_NOTES)" >&2; exit 1; }
	@bad=$$(find "$(SITE_NOTES)" -name '*.zr' | sort | while IFS= read -r f; do \
		$(MAZER) --check --strict "$$f" </dev/null >/dev/null 2>&1 || printf '  %s\n' "$$f"; \
	done); \
	if [ -n "$$bad" ]; then \
		printf 'deploy aborted: these notes fail `mazer --check --strict` (migrate them first; nothing was copied):\n%s\n' "$$bad" >&2; \
		exit 1; \
	fi

# Copies the wasm module, its glue and the equation search script to the site,
# after the notes check passes. Only these three files are copied; notes are
# never touched.
deploy: check-site-notes ## copies to adhyan.ca
	$(MAKE) wasm
	cp ./mazer-wasm/pkg/mazer_wasm_bg.wasm $(SITE)/
	cp ./mazer-wasm/pkg/mazer_wasm.js $(SITE)/
	cp ./mazer-wasm/pkg/mazer_search.js $(SITE)/
