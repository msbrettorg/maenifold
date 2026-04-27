# UpdateAgent

Updates an existing configured ACP agent entry.

## Parameters

- `name` (string, required): Agent name to update.
- `config` (object, required): Replacement agent configuration.

## Notes

The config shape is the same as `AddAgent`. This public HTTP-daemon tool
requires live config state and is advertised directly by `tools/list`.
