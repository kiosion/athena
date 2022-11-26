.PHONY: install, run, build, release, test, clean

SHELL:=/bin/bash

install:
	@cargo install

run:
	@cargo run --

build:
	@cargo build && \
	mkdir -p ./out/ && \
	cp ./target/debug/athena ./out/athena

release:
	@cargo build --release && \
	mkdir -p ./out/ && \
	cp ./target/release/athena ./out/athena

test:
	@cargo test

clean:
	@cargo clean && \
	rm -rf ./out/
