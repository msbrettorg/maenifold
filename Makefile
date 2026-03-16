.PHONY: frontend app web-assets web-build

frontend:
	cd vendor/acp-ui && if command -v bun >/dev/null 2>&1; then bun install && bun run build; else npm install && npm run build; fi

app: frontend
	cargo build --release -p maenifold-cli

web-assets:
	cd vendor/acp-ui && bun install && bun run build

web-build: web-assets
	cargo build --release -p maenifold-cli
