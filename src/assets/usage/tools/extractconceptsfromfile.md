# ExtractConceptsFromFile

Extracts `[[WikiLink]]` concepts from memory files to analyze graph nodes before synchronization.

## Parameters

- `identifier` (string, required): Memory FILE identifier (URI or title)

## Returns

```json
{
  "uri": "memory://research/ai/machine-learning",
  "concepts": ["machine-learning", "neural-networks", "deep-learning"],
  "count": 3
}
```

## Example

```json
{"identifier": "memory://machine-learning-fundamentals"}
```

Shows concepts like `[[Neural Networks]]`, `[[Deep Learning]]`, `[[Supervised Learning]]`.

## Use Cases

- **Pre-Sync Validation**: Verify files have `[[concepts]]` before graph rebuild
- **Debug Missing Concepts**: Check if concepts exist in source when BuildContext fails
- **Content Audit**: Analyze conceptual coverage across files

## Integration

- **Sync**: Preview what concepts Sync will process
- **BuildContext**: Verify concepts exist before traversal
- **WriteMemory/EditMemory**: Validate `[[concept]]` formatting

## Common Issues

**No concepts found**: File lacks `[[WikiLink]]` formatting. Add `[[Concept Name]]` to connect to graph.

**File not found**: Verify URI format (`memory://filename` not `memory:filename`), use ReadMemory to confirm existence.

**Fewer concepts than expected**: Check double bracket formatting `[[concept]]` (not `[concept]` or split across lines).
