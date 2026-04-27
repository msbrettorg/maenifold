# UpdateSettings

Deep-merges partial maenifold settings into `maenifold.json` and refreshes live
daemon settings.

## Parameters

- `settings` (object, required): Partial settings object to merge. Supported
  top-level areas include logging, sync, vector, debug, community, decay,
  teams monitor, gateway, Easter eggs, and environment passthrough settings.

## Example

```json
{
  "settings": {
    "gateway": {
      "intentResolver": {
        "enabled": false
      }
    }
  }
}
```

## Notes

This public HTTP-daemon tool requires live config state and is advertised
directly by `tools/list`.
