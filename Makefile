VERSION = 0.0.7
SOURCES = $(shell find src/** -name "*.rs") $(shell find tests/** -name "*.rs") Cargo.toml

all: build test doc ## run test, doc and build target

.nvim/.venv/bin/activate: .nvim/requirements.txt ## prepare the python environment.
	python -m venv .venv
	. .venv/bin/activate; .venv/bin/python -m pip install --upgrade pip
	. .venv/bin/activate; .venv/bin/pip install -r .nvim/requirements.txt

clean: ## remove all build files.
	cargo clean --quiet

build: $(SOURCES) ## build the rust code.
	cargo build --lib --quiet

test: ## run all the test cases.
	RUST_LOG=debug cargo --quiet test -- --nocapture

doc: $(SOURCES) ## create the rust and sphinx documentation.
	#cargo doc --no-deps --lib --quiet
	cargo doc --no-deps --lib --document-private-items --quiet

.PHONY: help

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

