<!-- Minimal local copy of the official ACP docs. Source: https://agentclientprotocol.com/protocol/session-modes -->

# Session Modes

> Switch between different agent operating modes

<Note>
  You can now use Session Config Options. Dedicated
  session mode methods will be removed in a future version of the protocol.
  Until then, you can offer both to clients for backwards compatibility.
</Note>

Agents can provide a set of modes they can operate in. Modes often affect the system prompts used, the availability of tools, and whether they request permission before running.

## Initial state

During Session Setup the Agent **MAY** return a list of modes it can operate in and the currently active mode.

Example modes:

- `ask` — Request permission before making any changes
- `architect` — Design and plan software systems without implementation
- `code` — Write and modify code with full tool access

## Setting the current mode

The current mode can be changed at any point during a session, whether the Agent is idle or generating a response.

### From the Client

Typically, Clients display the available modes to the user and allow them to change the current one, which they can do by calling the `session/set_mode` method.

### From the Agent

The Agent can also change its own mode and let the Client know by sending the `current_mode_update` session notification.

#### Exiting plan modes

A common case where an Agent might switch modes is from within a special “exit mode” tool that can be provided to the language model during plan/architect modes.

This “switch mode” tool will usually request permission before running, which it can do just like any other tool.
