# Maenifold UI

A desktop and web client for the [Agent Client Protocol (ACP)](https://agentclientprotocol.com/). Connect to AI coding agents like GitHub Copilot, Claude Code, Gemini CLI, Qwen Code, Codex CLI, OpenCode, OpenClaw, Kiro CLI, Hermes Agent, and any ACP-compatible agent from a unified interface.

> **Attribution:** Maenifold UI is a copy-and-own, rebranded fork of [`formulahendry/acp-ui`](https://github.com/formulahendry/acp-ui) by **Jun Han (formulahendry)**, used under the MIT License. The original copyright notice and license are retained in [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE). This client is a standalone ACP UI and has no coupling to the maenifold backend.

## ✨ Features

- **Multi-Agent Support** — Connect to any ACP-compatible agent
- **Remote agents over WebSocket** — Talk to agents on another machine via `ws://` / `wss://`
- **Web app** — Run in any modern browser; supports remote WebSocket agents
- **Session Management** — Create, resume, and manage conversation sessions
- **Rich Chat Interface** — Markdown rendering, syntax highlighting, tool call visualization
- **Slash Commands** — Quick access to agent capabilities with `/command` syntax
- **Permission Controls** — Approve or deny agent actions before execution
- **Session Modes** — Switch between agent modes (ask, code, architect, etc.)
- **Model Picker** — Select from available AI models (unstable API)
- **Agent Thinking** — View the agent's reasoning process (collapsible)
- **Environment Variables** — Configure per-agent environment variables (API keys, settings)
- **Traffic Monitor** — Debug and inspect ACP protocol messages in real-time
- **Hot-Reload Config** — Edit agent configurations without restarting (desktop)
- **Cross-Platform Desktop** — Windows, macOS (ARM/Intel), Linux (x64/ARM64), plus the web build

## 🎯 Default Agents

Maenifold UI comes pre-configured with these agents:

| Agent | Package |
|-------|---------|
| [GitHub Copilot](https://github.com/github/copilot-language-server-release?tab=readme-ov-file#agent-client-protocol-acp-preview) | `@github/copilot-language-server` |
| [Claude Code](https://github.com/anthropics/claude-code) | `@agentclientprotocol/claude-agent-acp` |
| [Gemini CLI](https://github.com/google-gemini/gemini-cli) | `@google/gemini-cli` |
| [Qwen Code](https://github.com/QwenLM/qwen-code) | `@qwen-code/qwen-code` |
| [Auggie CLI](https://github.com/AugmentCode/auggie) | `@augmentcode/auggie` |
| [Qoder CLI](https://github.com/qoder-ai/qodercli) | `@qoder-ai/qodercli` |
| [Codex CLI](https://github.com/zed-industries/codex-acp) | `@zed-industries/codex-acp` |
| [OpenCode](https://github.com/opencode-ai/opencode) | `opencode-ai` |
| [OpenClaw](https://github.com/nicobailon/openclaw) | `openclaw` |
| [Kiro CLI](https://github.com/aws/kiro) | `kiro-cli` |
| [Hermes Agent](https://github.com/nichochar/hermes) | `hermes` |

## 🛠️ Configuration

Agent configurations are stored in:

| Platform | Path |
|----------|------|
| Windows | `%APPDATA%\maenifold-ui\agents.json` |
| macOS | `~/Library/Application Support/maenifold-ui/agents.json` |
| Linux | `~/.config/maenifold-ui/agents.json` |
| Web | Browser `localStorage` — managed via Settings UI |

### Example Configuration

```json
{
  "agents": {
    "GitHub Copilot": {
      "command": "npx",
      "args": ["@github/copilot-language-server@latest", "--acp"],
      "env": {}
    },
    "Claude Code": {
      "command": "npx",
      "args": ["@agentclientprotocol/claude-agent-acp@latest"],
      "env": {
        "ANTHROPIC_API_KEY": "sk-ant-..."
      }
    }
  }
}
```

> **Note**: Environment variables are passed to the agent process on startup. Use these for API keys, custom settings, or overriding default behavior.

### Remote agents over WebSocket

For agents running on another machine, use the `websocket` transport instead of `command`:

```json
{
  "agents": {
    "Copilot CLI (remote)": {
      "transport": "websocket",
      "url": "wss://acp.example.com/v1",
      "headers": { "Authorization": "Bearer YOUR_TOKEN" }
    }
  }
}
```

Both `ws://` (cleartext, for LAN / Dev Tunnels) and `wss://` (TLS) are accepted. `Authorization: Bearer <token>` is propagated as a WebSocket subprotocol because browser/WebView WebSocket APIs cannot set custom HTTP headers.

> **Note**: Filesystem RPCs (`fs/read_text_file`, `fs/write_text_file`) are only available on Tauri desktop (Windows, macOS, Linux). On the web client the capabilities are advertised as `false` and any incoming `fs/*` request from the agent is rejected with JSON-RPC `-32601 Method not found`. For remote agents the working directory path is interpreted on the **agent's host**, not on the client device.

## 📖 Usage

1. **Select an Agent** — Choose from the dropdown in the sidebar.
2. **Set Working Directory** — Pick a folder on desktop, or type an absolute path on web. The path is interpreted on the **agent's host**, not your device.
3. **Create Session** — Choose **New Session** to start chatting.
4. **Use Slash Commands** — Type `/` to see available commands.
5. **Resume Sessions** — Select a saved session in the sidebar to resume.

## 🚀 Development

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.70+
- Platform-specific build tools (see [Tauri Prerequisites](https://tauri.app/start/prerequisites/))

### Setup

```bash
# Install dependencies
npm install

# Run in development mode (Tauri desktop)
npm run tauri dev
```

### Build for Production (Desktop)

```bash
npm run tauri build
```

### Building / running the web app

The web app uses the same Vue 3 frontend, with the Tauri runtime swapped out for browser-native APIs (WebSocket, `localStorage`). It only supports remote agents over `ws://` / `wss://`.

```sh
# Dev server with HMR (default port 5173)
npm run dev:web

# Production build → dist-web/
npm run build:web

# Serve dist-web/ locally to verify the production bundle
npm run preview:web
```

## 🔗 Links

- [Agent Client Protocol](https://agentclientprotocol.com/)
- [Tauri Documentation](https://tauri.app/)
- [Upstream project: formulahendry/acp-ui](https://github.com/formulahendry/acp-ui)

## 📄 License

MIT License — see [`LICENSE`](LICENSE). Original work © 2026 Jun Han (formulahendry). See [`NOTICE`](NOTICE) for attribution details.
