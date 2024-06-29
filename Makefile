VERSION = 0.0.7
SOURCES = $(shell find src/** -name "*.rs") $(shell find tests/** -name "*.rs") Cargo.toml

all: build test doc ## run test, doc and build target

clean: ## remove all build files.
	cargo clean

build: $(SOURCES) ## build the rust code.
	cargo build --lib

test: ## run all the test cases.
	RUST_LOG=debug cargo test -- --nocapture

doc: $(SOURCES) ## create the rust and sphinx documentation.
	cargo doc --no-deps --lib

.PHONY: help

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

