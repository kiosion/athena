SHELL:=/bin/bash

install:
	@cargo install

run:
	@cargo run --

build:
	@cargo build && \
	mkdir -p ./build/ && \
	cp ./target/debug/athena ./build/athena

release:
	@cargo build --release && \
	mkdir -p ./build/ && \
	cp ./target/release/athena ./build/athena

clean:
	@cargo clean && \
	rm -rf ./build/
