---
name: mcp:fetch
description: A Model Context Protocol server that provides web content fetching capabilities.
---

A Model Context Protocol server that provides web content fetching capabilities. This server enables LLMs to retrieve and process content from web pages, converting HTML to markdown for easier consumption.

Caution: This server can access local/internal IP addresses and may represent a security risk. Exercise caution when using this MCP server to ensure this does not expose any sensitive data.

The fetch tool will truncate the response, but by using the start_index argument, you can specify where to start the content extraction. This lets models read a webpage in chunks, until they find the information they need.

## Available Tools

fetch - Fetches a URL from the internet and extracts its contents as markdown.
- url (string, required): URL to fetch
- max_length (integer, optional): Maximum number of characters to return (default: 5000)
- start_index (integer, optional): Start content from this character index (default: 0)
- raw (boolean, optional): Get raw content without markdown conversion (default: false)
