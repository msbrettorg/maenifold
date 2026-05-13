# MCP long text and audio fixture

This directory provides deterministic local TypeScript content helpers for desktop tests that need large MCP-style text blocks or small audio payloads without contacting an external MCP server.

See gate task T-303.14 for the fixture spec. See `__tests__/index.test.ts` for the shape contract.

Run the contract with:

```bash
cd apps/desktop
npx vitest run test-fixtures/mcp-long-audio/
```

SWE acceptance criteria:

- Export `getLongText(sizeKb: number)` from `index.ts`.
  - Return MCP text content blocks shaped as `{ type: 'text', text: string }`.
  - Keep returned UTF-8 byte length approximately `sizeKb * 1000`.
- Export `getSineWaveAudio(seconds: number)` from `index.ts`.
  - Return MCP audio content blocks shaped as `{ type: 'audio', data: string, mimeType: 'audio/wav' }`.
  - Generate a base64-encoded 16-bit PCM mono WAV with a valid RIFF/WAVE header.
- Do not add dependencies or change desktop package, TypeScript, or Vitest configuration.
