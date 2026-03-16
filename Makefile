.PHONY: build release

build:
	cargo build -p maenifold-cli

release:
	cargo build --release -p maenifold-cli
