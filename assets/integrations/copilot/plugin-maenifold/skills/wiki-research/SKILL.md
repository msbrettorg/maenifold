---
name: wiki-research
description: Download Wikipedia articles, convert hyperlinks to [[WikiLinks]], and ingest into maenifold memory for graph-grounded research. Use when users ask to "research a topic from Wikipedia", "ingest wiki articles", "download Wikipedia into memory", "build a research corpus", or need to seed the knowledge graph with authoritative reference material from any domain.
---

# Wikipedia Research Ingest

Download Wikipedia articles, convert inline hyperlinks to `[[WikiLinks]]`, and write them to `memory://` via the maenifold CLI. Produces a JSON concept index mapping every converted concept to its original Wikipedia URL.

Templated from `persistence.ts` patterns:
- Reads `~/maenifold/memory/.wikilink-filter.json` for hub concept filtering
- Uses NFKC unicode normalization (mirrors `_sanitizeForStorage`)
- Invokes maenifold CLI with `--json` and parses structured output (mirrors `_parseMaenifoldOutput`)
- Strips hub WikiLinks that would pollute the graph (mirrors `_filterWikiLinks`)

## Commands

### Single article

```bash
python3 scripts/wiki_ingest.py single \
    --url "https://en.wikipedia.org/wiki/Bayesian_inference" \
    --folder "research/topic-name" \
    --tags "tag1,tag2"
```

Returns JSON with the article content, concept map, and stats.

### Batch from manifest

The manifest is a JSON object keyed by category, each containing an array of `{id, title, url, tags}` entries.

```json
{
  "category-name": [
    {
      "id": "article-id",
      "title": "Article Title",
      "url": "https://en.wikipedia.org/wiki/Article_Name",
      "tags": ["tag1", "tag2"]
    }
  ]
}
```

```bash
python3 scripts/wiki_ingest.py batch \
    --manifest path/to/manifest.json \
    --base-folder "research/domain" \
    --output /tmp/results.json
```

### Extract concept index

After ingest, extract the deduplicated concept-to-URL mapping:

```bash
python3 scripts/wiki_ingest.py concepts --results /tmp/results.json
```

Produces a JSON array of `{concept, urls[], source_articles[]}` — the full index of Wikipedia concepts discovered during ingest, mapped to their original URLs. Use this to:
- Recursively fetch linked articles for deeper coverage
- Verify concept coverage across a domain
- Build manifests for follow-up ingestion passes

### Dry run

Add `--dry-run` to any command to skip writes:

```bash
python3 scripts/wiki_ingest.py batch --manifest manifest.json --base-folder "research/domain" --dry-run
```

## Pipeline

Per article, the script:

1. **Downloads** via Wikipedia parse API (`action=parse&prop=text&redirects=1`)
2. **Enumerates** all `<a href="/wiki/...">` hyperlinks
3. **Converts** to `[[wikilinks]]` (NFKD decomposition, possessive stripping, kebab-case normalization)
4. **Filters** hub concepts from `.wikilink-filter.json` (strips brackets, keeps plain text)
5. **Appends** a `## Source` section with lineage (URL + access date)
6. **Writes** to `memory://<folder>/<title>` via `maenifold --tool WriteMemory --json`
7. **Returns** JSON mapping: `{concept, display, url}` for every converted link

## When to Use

- Seeding a new domain in the knowledge graph from authoritative sources
- Building research corpora before workflow authoring, sprint planning, or deep dives
- Ingesting specification articles that serve as the canonical reference for implementation work
- Expanding graph coverage by following the concept index from a previous ingest
