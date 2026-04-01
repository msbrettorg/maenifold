#!/usr/bin/env python3
from __future__ import annotations

import re
from pathlib import Path


DONE_DIR = Path("done")
START_MAJOR = 80
HEADING_RE = re.compile(r"^# (.+)$", re.MULTILINE)
REQUIREMENT_RE = re.compile(r"\*\*Requirement\*\*:\s*((?:FR|NFR)-\d+(?:\.\d+)?)\b")
FRONTMATTER_KEY_RE = re.compile(r"^([A-Za-z0-9_-]+):\s*(.*)$")
ID_PREFIX_RE = re.compile(r"^[A-Za-z0-9]+(?:-[A-Za-z0-9.]+)+(?:\s*(?:—|:|-)\s*)?")


def split_frontmatter(text: str) -> tuple[dict[str, str], str]:
    if not text.startswith("---"):
        return {}, text

    lines = text.splitlines(keepends=True)
    if not lines or lines[0].strip() != "---":
        return {}, text

    for index in range(1, len(lines)):
        if lines[index].strip() == "---":
            frontmatter: dict[str, str] = {}
            for raw_line in lines[1:index]:
                line = raw_line.rstrip("\r\n")
                match = FRONTMATTER_KEY_RE.match(line)
                if not match:
                    continue
                key, value = match.groups()
                value = value.strip()
                if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
                    value = value[1:-1]
                frontmatter[key] = value
            body = "".join(lines[index + 1 :])
            return frontmatter, body

    return {}, text


def extract_title(body: str, fallback_slug: str) -> str:
    match = HEADING_RE.search(body)
    if match:
        title = ID_PREFIX_RE.sub("", match.group(1).strip()).strip()
        if title:
            return title
    return fallback_slug.replace("-", " ").replace("_", " ").strip().title()


def quote_yaml(value: str) -> str:
    if value == "":
        return '""'
    if (
        value != value.strip()
        or ":" in value
        or "#" in value
        or '"' in value
        or "'" in value
        or value.startswith(("-", "?", "@", "`", "!", "&", "*", "{", "}", "[", "]", ",", "|", ">", "%"))
    ):
        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'
    return value


def build_frontmatter(fields: list[tuple[str, str]]) -> str:
    lines = ["---\n"]
    for key, value in fields:
        lines.append(f"{key}: {quote_yaml(value)}\n")
    lines.append("---\n")
    return "".join(lines)


def main() -> None:
    slug_legacy = sorted([path for path in DONE_DIR.glob("*.md") if not path.name.startswith("T-")])
    t_st_legacy = sorted(DONE_DIR.glob("T-ST-*.md"))
    legacy_files = [*slug_legacy, *t_st_legacy]

    fixed = 0
    clean = 0
    mappings: list[tuple[str, str, str]] = []

    for index, path in enumerate(legacy_files, start=1):
        new_id = f"T-{START_MAJOR}.{index}"
        original_text = path.read_text(encoding="utf-8")
        frontmatter, body = split_frontmatter(original_text)
        old_id = frontmatter.get("id", "(missing)")
        title = extract_title(body, path.stem)
        status = frontmatter.get("status", "done")
        pipeline = frontmatter.get("pipeline", "")
        requirement_match = REQUIREMENT_RE.search(body)
        requirement = requirement_match.group(1) if requirement_match else None

        fields: list[tuple[str, str]] = [
            ("id", new_id),
            ("title", title),
            ("status", status),
            ("pipeline", pipeline),
        ]
        if requirement:
            fields.append(("requirement", requirement))

        new_text = build_frontmatter(fields) + body
        mappings.append((path.name, old_id, new_id))

        if new_text == original_text:
            clean += 1
            print(f"{path}: already clean ({old_id} -> {new_id})")
            continue

        path.write_text(new_text, encoding="utf-8")
        fixed += 1
        changes = [f"id -> {new_id}", f"title -> {title}"]
        if requirement and frontmatter.get("requirement") != requirement:
            changes.append(f"requirement -> {requirement}")
        elif not requirement and "requirement" in frontmatter:
            changes.append("requirement removed")
        print(f"{path}: " + ", ".join(changes))

    print(f"Summary: fixed={fixed} already_clean={clean} total={len(legacy_files)}")
    print("Mappings:")
    for filename, old_id, new_id in mappings:
        print(f"{filename}: {old_id} -> {new_id}")


if __name__ == "__main__":
    main()
