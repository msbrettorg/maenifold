.PHONY: web-assets web-build

web-assets:
	cd vendor/acp-ui && bun install && bun run build

web-build: web-assets
	cargo build --release -p maenifold-cli
