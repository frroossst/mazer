.PHONY: repl build clean help

help: ## print this message
	@echo "Usage: make [command]"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "\033[1;33m%-30s\033[0m %s\n", $$1, $$2}'

repl: build ## run in REPL mode
	rlwrap cargo run -- --repl

build: ## build the project
	cargo build

clean: ## clean the project
	rm -rf target/

