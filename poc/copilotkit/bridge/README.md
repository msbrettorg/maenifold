# maenifold CopilotKit Bridge

Small POC bridge that translates maenifold ACP WebSocket updates into AG-UI Server-Sent Events for CopilotKit.

## Prerequisites

- maenifold daemon running (default port 8080)
- Node.js 18+

## Setup

```bash
cd poc/copilotkit/bridge
npm install
```

## Run

```bash
npm run dev
```

Or:

```bash
npm start
```

## Environment

```bash
MAENIFOLD_PORT=8080      # daemon port
MAENIFOLD_AGENT=copilot  # agent name
BRIDGE_PORT=4000         # bridge listen port
```

The bridge listens on `http://127.0.0.1:4000` by default. `GET /info` returns basic service metadata, and `POST /` accepts a CopilotKit RunAgent-shaped JSON body and returns an AG-UI SSE stream.
