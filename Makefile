.PHONY: dev release test

FEATURES ?= enhanced
CARGO_FEATURE_FLAGS := --no-default-features --features $(FEATURES)
BIN_NAME := rockbot
ROOT_BIN := ./$(BIN_NAME)
DEBUG_BIN := target/debug/$(BIN_NAME)
RELEASE_BIN := target/release/$(BIN_NAME)

dev:
	rm -f $(ROOT_BIN)
	cargo build $(CARGO_FEATURE_FLAGS)
	cp $(DEBUG_BIN) $(ROOT_BIN)

release:
	rm -f $(ROOT_BIN)
	cargo build --release $(CARGO_FEATURE_FLAGS)
	cp $(RELEASE_BIN) $(ROOT_BIN)

test:
	cargo test --workspace --lib --bins --tests $(CARGO_FEATURE_FLAGS)
