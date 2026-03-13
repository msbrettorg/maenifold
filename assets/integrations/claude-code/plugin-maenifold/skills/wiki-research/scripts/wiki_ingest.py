#!/usr/bin/env python3
"""Download Wikipedia articles, convert hyperlinks to [[WikiLinks]], and ingest into maenifold memory.

Templated from persistence.ts patterns:
- _read_wikilink_filter() mirrors _readWikilinkFilter()
- _filter_wikilinks() mirrors _filterWikiLinks()
- _sanitize_for_storage() mirrors _sanitizeForStorage()
- _parse_maenifold_output() mirrors _parseMaenifoldOutput()
- CLI invocation mirrors Bun.spawn(["maenifold", ...]) with --json

Usage:
    # Single article
    python3 wiki_ingest.py single \\
        --url "https://en.wikipedia.org/wiki/Bayesian_inference" \\
        --folder "research/workflows/reasoning" \\
        --tags scientific-method,bayesian-reasoning

    # Batch from manifest
    python3 wiki_ingest.py batch \\
        --manifest /tmp/workflow_articles.json \\
        --base-folder "research/workflows"

    # Dry run (no writes)
    python3 wiki_ingest.py batch \\
        --manifest /tmp/workflow_articles.json \\
        --base-folder "research/workflows" \\
        --dry-run
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import unicodedata
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any


# ---------------------------------------------------------------------------
# Constants (mirrors persistence.ts _CLI_TIMEOUT_MS, _PAYLOAD_MAX_CHARS)
# ---------------------------------------------------------------------------

_MAENIFOLD_ROOT = os.environ.get("MAENIFOLD_ROOT", os.path.expanduser("~/maenifold"))
_FILTER_PATH = os.path.join(_MAENIFOLD_ROOT, "memory", ".wikilink-filter.json")
_CLI_TIMEOUT_S = 30
_PAYLOAD_MAX_CHARS = 50_000

# Patterns matching persistence.ts
_WIKILINK_RE = re.compile(r"\[\[([^\]]+)\]\]")


# ---------------------------------------------------------------------------
# WikiLink filter (mirrors persistence.ts _readWikilinkFilter)
# ---------------------------------------------------------------------------

def _read_wikilink_filter() -> set[str]:
    """Read the hub concept filter. Returns a set of normalized concept names."""
    try:
        with open(_FILTER_PATH, "r", encoding="utf-8") as f:
            obj = json.load(f)
        return set(obj.keys())
    except FileNotFoundError:
        return set()


def _filter_wikilinks(
    text: str, filter_set: set[str]
) -> tuple[str, list[str], int]:
    """Strip hub WikiLinks, keeping plain text. Mirrors persistence.ts _filterWikiLinks.

    Returns (cleaned_text, stripped_terms, surviving_count).
    """
    stripped_terms: list[str] = []
    surviving_count = 0

    def replacer(match: re.Match) -> str:
        nonlocal surviving_count
        inner = match.group(1)
        normalized = inner.strip().lower().replace(" ", "-")
        # Normalize same way as persistence.ts
        normalized = re.sub(r"\s+", "-", normalized)
        if normalized in filter_set:
            stripped_terms.append(inner.strip())
            return inner.strip()  # strip brackets, keep plain text
        surviving_count += 1
        return match.group(0)  # keep as-is

    cleaned = _WIKILINK_RE.sub(replacer, text)
    return cleaned, stripped_terms, surviving_count


# ---------------------------------------------------------------------------
# Sanitization (mirrors persistence.ts _sanitizeForStorage)
# ---------------------------------------------------------------------------

def _sanitize_for_storage(text: str) -> str:
    """Normalize unicode and strip control chars. Mirrors persistence.ts _sanitizeForStorage."""
    text = unicodedata.normalize("NFKC", text)
    # Remove control characters (matching the JS regex range)
    text = re.sub(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]", "", text)
    return text.strip()


# ---------------------------------------------------------------------------
# CLI invocation (mirrors persistence.ts Bun.spawn + _parseMaenifoldOutput)
# ---------------------------------------------------------------------------

def _parse_maenifold_output(stdout: str) -> dict[str, Any]:
    """Parse maenifold --json output. Mirrors persistence.ts _parseMaenifoldOutput."""
    parsed = json.loads(stdout.strip())
    if not parsed.get("success"):
        error = parsed.get("error", {})
        msg = error.get("message", "Unknown maenifold error")
        raise RuntimeError(msg)
    data = parsed.get("data", {})
    if not data.get("uri"):
        raise RuntimeError("maenifold returned success but no uri")
    return data


def _write_to_memory(
    title: str,
    content: str,
    folder: str,
    tags: list[str] | None = None,
    dry_run: bool = False,
) -> dict[str, Any] | None:
    """Write content to maenifold memory via CLI. Returns parsed data or None for dry run."""
    payload: dict[str, Any] = {
        "title": title,
        "content": content,
        "folder": folder,
    }
    if tags:
        payload["tags"] = tags

    payload_json = json.dumps(payload, ensure_ascii=False)

    if len(payload_json) > _PAYLOAD_MAX_CHARS:
        # Truncate content to fit (preserve Source section at the end)
        source_marker = "\n## Source\n"
        source_idx = content.rfind(source_marker)
        if source_idx > 0:
            source_section = content[source_idx:]
            available = _PAYLOAD_MAX_CHARS - len(json.dumps({
                "title": title, "content": "", "folder": folder, "tags": tags or []
            })) - len(source_section) - 200  # margin
            truncated_body = content[:available] + "\n\n[TRUNCATED]\n"
            payload["content"] = truncated_body + source_section
            payload_json = json.dumps(payload, ensure_ascii=False)

    if dry_run:
        print(f"  [DRY RUN] Would write: {folder}/{title} ({len(payload_json)} chars)")
        return None

    result = subprocess.run(
        ["maenifold", "--tool", "WriteMemory", "--payload", payload_json, "--json"],
        capture_output=True,
        text=True,
        timeout=_CLI_TIMEOUT_S,
    )

    if result.returncode != 0:
        raise RuntimeError(
            f"maenifold exit {result.returncode}: {result.stderr or result.stdout}"
        )

    return _parse_maenifold_output(result.stdout)


# ---------------------------------------------------------------------------
# Wikipedia fetch
# ---------------------------------------------------------------------------

_WIKI_PARSE_API = "https://en.wikipedia.org/w/api.php"


def _fetch_wikipedia_html(article_title: str) -> str:
    """Fetch Wikipedia article HTML via the parse API (handles redirects)."""
    params = urllib.parse.urlencode({
        "action": "parse",
        "page": article_title,
        "prop": "text",
        "format": "json",
        "redirects": "1",
        "disableeditsection": "1",
    })
    url = f"{_WIKI_PARSE_API}?{params}"

    req = urllib.request.Request(url, headers={
        "User-Agent": "maenifold-wiki-ingest/1.0 (https://github.com/msbrettorg/maenifold)"
    })

    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read().decode("utf-8"))

    if "error" in data:
        raise RuntimeError(f"Wikipedia API error: {data['error'].get('info', str(data['error']))}")

    return data["parse"]["text"]["*"]


def _extract_article_title_from_url(url: str) -> str:
    """Extract article title from a Wikipedia URL."""
    parsed = urllib.parse.urlparse(url)
    path = parsed.path
    # /wiki/Article_Name -> Article_Name
    if "/wiki/" in path:
        return urllib.parse.unquote(path.split("/wiki/")[-1])
    raise ValueError(f"Cannot extract article title from URL: {url}")


# ---------------------------------------------------------------------------
# HTML → Markdown with WikiLink conversion
# ---------------------------------------------------------------------------

# Links to skip (non-article Wikipedia links)
_SKIP_LINK_PATTERNS = [
    "#cite_note", "#cite_ref", "File:", "Category:", "Special:",
    "Wikipedia:", "Template:", "Help:", "Talk:", "Portal:",
    ".svg", ".png", ".jpg", ".gif", ".jpeg",
    "/w/index.php",
]


def _normalize_concept(text: str) -> str:
    """Normalize display text to a WikiLink concept name.

    Matches persistence.ts normalization: .trim().toLowerCase().replace(/\\s+/g, '-')
    Plus unicode NFKD decomposition for non-ASCII chars.
    """
    # NFKD decomposition to handle accented characters
    nfkd = unicodedata.normalize("NFKD", text)
    ascii_text = nfkd.encode("ascii", "ignore").decode("ascii")

    # Strip possessives
    concept = re.sub(r"'s$", "", ascii_text)
    # Strip parentheticals
    concept = re.sub(r"\s*\([^)]*\)\s*", " ", concept)
    # Lowercase, strip non-alphanumeric
    concept = concept.lower().strip()
    concept = re.sub(r"[^a-z0-9\s-]", "", concept)
    # Spaces to hyphens
    concept = re.sub(r"\s+", "-", concept)
    # Collapse multiple hyphens
    concept = re.sub(r"-+", "-", concept)
    concept = concept.strip("-")

    return concept


def _html_to_markdown_with_wikilinks(
    html: str,
    filter_set: set[str],
) -> tuple[str, list[dict[str, str]]]:
    """Convert Wikipedia HTML to markdown, replacing article hyperlinks with [[WikiLinks]].

    Returns:
        (markdown_text, concept_map) where concept_map is a list of
        {"concept": "...", "display": "...", "url": "..."} dicts.
    """
    concept_map: list[dict[str, str]] = []
    seen_concepts: set[str] = set()

    # Step 1: Remove unwanted elements (references, edit links, navboxes, etc.)
    # Remove reference tags
    html = re.sub(r'<sup[^>]*class="reference"[^>]*>.*?</sup>', "", html, flags=re.DOTALL)
    # Remove [edit] links
    html = re.sub(r'<span class="mw-editsection">.*?</span>', "", html, flags=re.DOTALL)
    # Remove navboxes, sidebars, metadata
    html = re.sub(r'<div[^>]*class="[^"]*(?:navbox|sidebar|metadata|mw-empty)[^"]*"[^>]*>.*?</div>', "", html, flags=re.DOTALL)
    # Remove style and script tags
    html = re.sub(r"<style[^>]*>.*?</style>", "", html, flags=re.DOTALL)
    html = re.sub(r"<script[^>]*>.*?</script>", "", html, flags=re.DOTALL)
    # Remove HTML comments
    html = re.sub(r"<!--.*?-->", "", html, flags=re.DOTALL)

    # Step 2: Convert <a> tags to [[WikiLinks]] or plain text
    def convert_link(match: re.Match) -> str:
        attrs = match.group(1)
        display = match.group(2)

        # Clean display text of nested HTML
        display = re.sub(r"<[^>]+>", "", display).strip()
        if not display:
            return ""

        # Extract href
        href_match = re.search(r'href="([^"]*)"', attrs)
        if not href_match:
            return display

        href = href_match.group(1)

        # Skip non-article links
        for pattern in _SKIP_LINK_PATTERNS:
            if pattern in href:
                return display

        # Only convert /wiki/ links (internal Wikipedia article links)
        if not href.startswith("/wiki/"):
            return display

        # Build the concept name
        concept = _normalize_concept(display)
        if not concept or len(concept) < 2:
            return display

        # Check against filter set
        if concept in filter_set:
            return display

        # Build the full Wikipedia URL for the mapping
        wiki_url = f"https://en.wikipedia.org{href}"

        # Track in concept map (deduplicate)
        if concept not in seen_concepts:
            seen_concepts.add(concept)
            concept_map.append({
                "concept": concept,
                "display": display,
                "url": wiki_url,
            })

        return f"[[{concept}]]"

    # Match <a ...>...</a> (non-greedy)
    html = re.sub(r"<a\s([^>]*)>(.*?)</a>", convert_link, html, flags=re.DOTALL)

    # Step 3: Convert remaining HTML to markdown-ish text
    # Headers
    for level in range(6, 0, -1):
        prefix = "#" * level
        html = re.sub(
            rf"<h{level}[^>]*>(.*?)</h{level}>",
            lambda m, p=prefix: f"\n{p} {re.sub(r'<[^>]+>', '', m.group(1)).strip()}\n",
            html,
            flags=re.DOTALL,
        )

    # Bold/italic
    html = re.sub(r"<b>(.*?)</b>", r"**\1**", html, flags=re.DOTALL)
    html = re.sub(r"<strong>(.*?)</strong>", r"**\1**", html, flags=re.DOTALL)
    html = re.sub(r"<i>(.*?)</i>", r"*\1*", html, flags=re.DOTALL)
    html = re.sub(r"<em>(.*?)</em>", r"*\1*", html, flags=re.DOTALL)

    # Lists
    html = re.sub(r"<li[^>]*>(.*?)</li>", r"\n- \1", html, flags=re.DOTALL)

    # Paragraphs and line breaks
    html = re.sub(r"<p[^>]*>", "\n\n", html)
    html = re.sub(r"</p>", "", html)
    html = re.sub(r"<br\s*/?>", "\n", html)
    html = re.sub(r"<hr\s*/?>", "\n---\n", html)

    # Blockquotes
    html = re.sub(r"<blockquote[^>]*>(.*?)</blockquote>",
                  lambda m: "\n> " + m.group(1).strip().replace("\n", "\n> ") + "\n",
                  html, flags=re.DOTALL)

    # Tables — simplify to pipe-delimited text
    html = re.sub(r"<th[^>]*>(.*?)</th>", r"| \1 ", html, flags=re.DOTALL)
    html = re.sub(r"<td[^>]*>(.*?)</td>", r"| \1 ", html, flags=re.DOTALL)
    html = re.sub(r"<tr[^>]*>", "\n", html)
    html = re.sub(r"</tr>", " |", html)

    # Strip all remaining HTML tags
    html = re.sub(r"<[^>]+>", "", html)

    # Decode HTML entities
    html = html.replace("&amp;", "&")
    html = html.replace("&lt;", "<")
    html = html.replace("&gt;", ">")
    html = html.replace("&nbsp;", " ")
    html = html.replace("&ndash;", "-")
    html = html.replace("&mdash;", "--")
    html = html.replace("&quot;", '"')
    html = html.replace("&#39;", "'")

    # Collapse excessive whitespace
    html = re.sub(r"\n{3,}", "\n\n", html)
    html = re.sub(r" {2,}", " ", html)

    return html.strip(), concept_map


# ---------------------------------------------------------------------------
# Ingest pipeline
# ---------------------------------------------------------------------------

def ingest_article(
    url: str,
    title: str,
    folder: str,
    tags: list[str],
    filter_set: set[str],
    dry_run: bool = False,
) -> dict[str, Any]:
    """Ingest a single Wikipedia article.

    Returns a result dict with:
        - id, title, url, folder
        - uri (memory:// URI if written)
        - concept_map: list of {concept, display, url}
        - stats: {total_links, surviving_wikilinks, stripped_hub_terms}
        - error (if failed)
    """
    result: dict[str, Any] = {
        "id": title.lower().replace(" ", "-"),
        "title": title,
        "url": url,
        "folder": folder,
        "uri": None,
        "concept_map": [],
        "stats": {},
        "error": None,
    }

    try:
        # 1. Download the page
        article_title = _extract_article_title_from_url(url)
        html = _fetch_wikipedia_html(article_title)

        # 2+3. Enumerate hyperlinks and replace with [[WikiLinks]]
        markdown, concept_map = _html_to_markdown_with_wikilinks(html, filter_set)
        result["concept_map"] = concept_map

        # 4. Sanitize (mirrors persistence.ts _sanitizeForStorage)
        content = _sanitize_for_storage(markdown)

        # Apply wikilink filter pass (mirrors persistence.ts _filterWikiLinks)
        # This catches any hub concepts that slipped through
        content, stripped_terms, surviving = _filter_wikilinks(content, filter_set)

        result["stats"] = {
            "total_links_converted": len(concept_map),
            "surviving_wikilinks": surviving,
            "stripped_hub_terms": len(stripped_terms),
        }

        if surviving == 0:
            result["error"] = f"No WikiLinks survived filtering ({len(stripped_terms)} stripped)"
            return result

        # Add Source section (lineage requirement)
        content += f"\n\n## Source\n\n- [{title}]({url}) — Wikipedia, accessed {_today()}\n"
        content += "- Content type: direct extract with WikiLink conversion\n"

        # Insert into memory
        mem_title = title.replace(" ", "-").lower()
        mem_title = re.sub(r"[^a-z0-9-]", "", mem_title)
        data = _write_to_memory(
            title=mem_title,
            content=content,
            folder=folder,
            tags=tags,
            dry_run=dry_run,
        )
        if data:
            result["uri"] = data.get("uri")

    except Exception as exc:
        result["error"] = str(exc)

    return result


def _today() -> str:
    """Return today's date as YYYY-MM-DD."""
    from datetime import date
    return date.today().isoformat()


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

def cmd_single(args: argparse.Namespace) -> int:
    """Ingest a single Wikipedia article."""
    filter_set = _read_wikilink_filter()
    print(f"Loaded {len(filter_set)} hub concepts from wikilink filter")

    tags = [t.strip() for t in args.tags.split(",")] if args.tags else []

    result = ingest_article(
        url=args.url,
        title=args.title or _extract_article_title_from_url(args.url).replace("_", " "),
        folder=args.folder,
        tags=tags,
        filter_set=filter_set,
        dry_run=args.dry_run,
    )

    # Output JSON result
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 1 if result["error"] else 0


def cmd_batch(args: argparse.Namespace) -> int:
    """Batch ingest from a manifest file."""
    filter_set = _read_wikilink_filter()
    print(f"Loaded {len(filter_set)} hub concepts from wikilink filter")

    with open(args.manifest, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    results: list[dict[str, Any]] = []
    all_concepts: list[dict[str, str]] = []
    ok_count = 0
    fail_count = 0

    for category, articles in manifest.items():
        folder = f"{args.base_folder}/{category}"
        print(f"\n--- Category: {category} ({len(articles)} articles) -> {folder} ---")

        for article in articles:
            article_id = article["id"]
            article_title = article["title"]
            article_url = article["url"]
            article_tags = article.get("tags", [])

            print(f"  [{article_id}] {article_title}...")

            result = ingest_article(
                url=article_url,
                title=article_title,
                folder=folder,
                tags=article_tags,
                filter_set=filter_set,
                dry_run=args.dry_run,
            )

            results.append(result)

            if result["error"]:
                fail_count += 1
                print(f"    FAILED: {result['error']}")
            else:
                ok_count += 1
                uri = result.get("uri", "[dry run]")
                stats = result.get("stats", {})
                print(f"    OK: {uri} ({stats.get('total_links_converted', 0)} links, "
                      f"{stats.get('surviving_wikilinks', 0)} surviving)")

            # Accumulate concept map
            for concept_entry in result.get("concept_map", []):
                # Add category context
                enriched = {**concept_entry, "source_article": article_id, "category": category}
                all_concepts.append(enriched)

    # Summary
    print(f"\n=== SUMMARY: {ok_count} OK, {fail_count} FAILED out of {ok_count + fail_count} ===")

    # 5. Return JSON table mapping converted concepts to their OG Wikipedia URLs
    output = {
        "summary": {
            "total_articles": ok_count + fail_count,
            "ok": ok_count,
            "failed": fail_count,
            "total_unique_concepts": len({c["concept"] for c in all_concepts}),
        },
        "articles": results,
        "concept_index": all_concepts,
    }

    # Write output to file
    output_path = args.output or "/tmp/wiki_ingest_results.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)
        f.write("\n")
    print(f"\nResults written to: {output_path}")

    return 1 if fail_count > 0 else 0


def cmd_concepts(args: argparse.Namespace) -> int:
    """Extract and display the concept index from a previous ingest result."""
    with open(args.results, "r", encoding="utf-8") as f:
        data = json.load(f)

    concepts = data.get("concept_index", [])

    # Deduplicate by concept name, collecting all source URLs
    index: dict[str, dict[str, Any]] = {}
    for entry in concepts:
        concept = entry["concept"]
        if concept not in index:
            index[concept] = {
                "concept": concept,
                "urls": [],
                "source_articles": [],
            }
        url = entry.get("url", "")
        if url and url not in index[concept]["urls"]:
            index[concept]["urls"].append(url)
        src = entry.get("source_article", "")
        if src and src not in index[concept]["source_articles"]:
            index[concept]["source_articles"].append(src)

    # Output sorted
    sorted_index = sorted(index.values(), key=lambda x: x["concept"])
    print(json.dumps(sorted_index, indent=2, ensure_ascii=False))
    print(f"\n{len(sorted_index)} unique concepts across {len(concepts)} total links")

    return 0


# ---------------------------------------------------------------------------
# Argument parser
# ---------------------------------------------------------------------------

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Download Wikipedia articles, convert to [[WikiLinks]], ingest into maenifold memory."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # single
    single_p = subparsers.add_parser("single", help="Ingest a single Wikipedia article")
    single_p.add_argument("--url", required=True, help="Wikipedia article URL")
    single_p.add_argument("--title", help="Article title (auto-detected from URL if omitted)")
    single_p.add_argument("--folder", required=True, help="Memory folder (e.g. research/workflows/reasoning)")
    single_p.add_argument("--tags", help="Comma-separated tags")
    single_p.add_argument("--dry-run", action="store_true", help="Skip actual writes")
    single_p.set_defaults(func=cmd_single)

    # batch
    batch_p = subparsers.add_parser("batch", help="Batch ingest from a manifest JSON")
    batch_p.add_argument("--manifest", required=True, help="Path to workflow_articles.json")
    batch_p.add_argument("--base-folder", required=True, help="Base memory folder (e.g. research/workflows)")
    batch_p.add_argument("--output", help="Output results JSON path (default: /tmp/wiki_ingest_results.json)")
    batch_p.add_argument("--dry-run", action="store_true", help="Skip actual writes")
    batch_p.set_defaults(func=cmd_batch)

    # concepts
    concepts_p = subparsers.add_parser("concepts", help="Extract concept index from previous results")
    concepts_p.add_argument("--results", required=True, help="Path to wiki_ingest_results.json")
    concepts_p.set_defaults(func=cmd_concepts)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
