# CopilotKit + maenifold POC

Standalone proof-of-concept connecting CopilotKit's AG-UI frontend to maenifold's ACP backend.

## Architecture

```
React + CopilotKit ←── HTTP/SSE ──→ Node.js Bridge ←── ACP WebSocket ──→ Maenifold Daemon
(localhost:3000)                    (localhost:4000)                      (port 8080)
```

## Prerequisites

- maenifold daemon running (`maenifold` CLI)
- Node.js 18+

## Quick Start

### 1. Start the bridge
```bash
cd bridge
npm install
npm run dev
```

### 2. Start the app (new terminal)
```bash
cd app
npm install
npm run dev
```

### 3. Open http://localhost:3000

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| MAENIFOLD_PORT | 8080 | Daemon port |
| MAENIFOLD_AGENT | copilot | ACP agent name |
| BRIDGE_PORT | 4000 | Bridge listen port |
