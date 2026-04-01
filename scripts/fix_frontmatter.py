#!/usr/bin/env python3
from __future__ import annotations

import re
from pathlib import Path


ROOTS = [Path("todo"), Path("done")]
FILENAME_ID_RE = re.compile(r"^(T-\d+(?:\.\d+)?)")
HEADING_RE = re.compile(r"^# (.+)$", re.MULTILINE)
REQUIREMENT_RE = re.compile(r"\*\*Requirement\*\*:\s*((?:FR|NFR)-\d+(?:\.\d+)?)\b")
FRONTMATTER_KEY_RE = re.compile(r"^([A-Za-z0-9_-]+):\s*(.*)$")


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


def derive_title(body: str, task_id: str, filename: str) -> str:
    heading_match = HEADING_RE.search(body)
    if heading_match:
        heading = heading_match.group(1).strip()
        heading = re.sub(r"^T-[A-Za-z0-9.-]+(?:\s*(?:—|:|-)\s*)?", "", heading).strip()
        if heading:
            return heading

    slug = filename[:-3]
    if slug.startswith(task_id):
        slug = slug[len(task_id) :]
        slug = slug.lstrip("-_ ")
    slug = slug.replace("-", " ").replace("_", " ").strip()
    return slug.title() if slug else task_id


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


def process_file(path: Path) -> tuple[str, list[str]]:
    filename = path.name
    match = FILENAME_ID_RE.match(path.stem)
    if not match:
        return "skipped", [f"{path}: skipped (filename does not match task id pattern)"]

    task_id = match.group(1)
    original_text = path.read_text(encoding="utf-8")
    frontmatter, body = split_frontmatter(original_text)

    title = derive_title(body, task_id, filename)
    status = frontmatter.get("status")
    if not status:
        status = "done" if "done" in path.parts else "pending"

    pipeline = frontmatter.get("pipeline", "")
    requirement_match = REQUIREMENT_RE.search(body)
    requirement = requirement_match.group(1) if requirement_match else None

    fields: list[tuple[str, str]] = [
        ("id", task_id),
        ("title", title),
        ("status", status),
        ("pipeline", pipeline),
    ]
    if requirement:
        fields.append(("requirement", requirement))

    new_text = build_frontmatter(fields) + body

    changes: list[str] = []
    if frontmatter.get("id") != task_id:
        changes.append(f"id -> {task_id}")
    if frontmatter.get("title") != title:
        changes.append(f"title -> {title}")
    if "status" not in frontmatter:
        changes.append(f"status -> {status}")
    if requirement and frontmatter.get("requirement") != requirement:
        changes.append(f"requirement -> {requirement}")
    elif not requirement and "requirement" in frontmatter:
        changes.append("requirement removed")

    current_order = [key for key in ["id", "title", "status", "pipeline", "requirement"] if key in frontmatter]
    desired_order = [key for key, _ in fields]
    if current_order != desired_order:
        changes.append("field order normalized")

    if new_text == original_text:
        return "clean", [f"{path}: already clean"]

    path.write_text(new_text, encoding="utf-8")
    return "fixed", [f"{path}: " + ", ".join(changes or ["rewritten"])]


def main() -> None:
    fixed = 0
    skipped = 0
    clean = 0

    for root in ROOTS:
        for path in sorted(root.rglob("*.md")):
            status, logs = process_file(path)
            for line in logs:
                print(line)
            if status == "fixed":
                fixed += 1
            elif status == "skipped":
                skipped += 1
            else:
                clean += 1

    print(f"Summary: fixed={fixed} skipped={skipped} already_clean={clean}")


if __name__ == "__main__":
    main()
