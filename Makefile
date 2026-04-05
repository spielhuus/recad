VERSION = 0.0.7
SOURCES = $(shell find crates/** -name "*.rs") $(shell find crates/ -name "Cargo.toml")

ifeq ($(shell command -v cargo-insta),)
    $(error "cargo-insta" is not installed. Run: cargo install cargo-insta)
endif

all: build test doc ## run test, doc and build target

.nvim/.venv/bin/activate: .nvim/requirements.txt ## prepare the python environment.
	python -m venv .venv
	. .venv/bin/activate; .venv/bin/python -m pip install --upgrade pip
	. .venv/bin/activate; .venv/bin/pip install -r .nvim/requirements.txt

clean: ## remove all build files.
	cargo clean --quiet
	rm -rf logs

build: $(SOURCES) ## build the rust code.
	cargo build --lib --quiet

test: ## run all the test cases.
	RUST_LOG=debug cargo --quiet test -- --nocapture

doc: $(SOURCES) ## create the rust documentation.
	cargo doc --no-deps --quiet --package=recad --lib --all-features

.PHONY: help

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

