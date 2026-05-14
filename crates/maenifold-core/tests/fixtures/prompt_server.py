#!/usr/bin/env python3
"""MCP stdio server fixture exposing one prompt + one placeholder tool.

The placeholder tool exists only to satisfy gateway startup paths that require at
least one downstream tool. Prompt aggregation tests do not exercise the tool.

Args:
    server_id: Identifier returned in resolved prompt content.
    prompt_name: Name of the single prompt this server exposes.
"""
import json, sys

server_id = sys.argv[1]
prompt_name = sys.argv[2]
noop_tool_name = f"noop_{server_id}"


def read_message():
    while True:
        line = sys.stdin.readline()
        if line == "":
            return None
        line = line.strip()
        if line:
            return json.loads(line)


def write_message(message):
    sys.stdout.write(json.dumps(message, separators=(",", ":")) + "\n")
    sys.stdout.flush()


while True:
    message = read_message()
    if message is None:
        break
    method = message.get("method")
    msg_id = message.get("id")
    if method == "initialize":
        write_message({
            "jsonrpc": "2.0", "id": msg_id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"prompts": {}, "tools": {}},
                "serverInfo": {"name": server_id, "version": "0.0.1"},
            },
        })
    elif method == "notifications/initialized":
        continue
    elif method == "tools/list":
        write_message({
            "jsonrpc": "2.0", "id": msg_id,
            "result": {"tools": [{
                "name": noop_tool_name,
                "description": "no-op tool for fixture; exists to satisfy gateway startup",
                "inputSchema": {"type": "object"},
            }]},
        })
    elif method == "tools/call":
        write_message({
            "jsonrpc": "2.0", "id": msg_id,
            "result": {"content": [{"type": "text", "text": "ok"}], "isError": False},
        })
    elif method == "prompts/list":
        write_message({
            "jsonrpc": "2.0", "id": msg_id,
            "result": {
                "prompts": [{
                    "name": prompt_name,
                    "description": f"prompt from {server_id}",
                    "arguments": [],
                }],
            },
        })
    elif method == "prompts/get":
        params = message.get("params", {})
        if params.get("name") == prompt_name:
            write_message({
                "jsonrpc": "2.0", "id": msg_id,
                "result": {
                    "description": f"resolved by {server_id}",
                    "messages": [{
                        "role": "user",
                        "content": {"type": "text", "text": f"server-id:{server_id} prompt:{prompt_name}"},
                    }],
                },
            })
        else:
            write_message({
                "jsonrpc": "2.0", "id": msg_id,
                "error": {"code": -32602, "message": "unknown prompt"},
            })
    else:
        write_message({
            "jsonrpc": "2.0", "id": msg_id,
            "error": {"code": -32601, "message": "unknown method"},
        })
