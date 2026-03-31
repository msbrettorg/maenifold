#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Aggregate Rust quality artifacts into JSON/Markdown summaries."
    )
    parser.add_argument("--repo-root", default=".", help="Repository root for path normalization")
    parser.add_argument("--lcov", required=True, help="Path to an LCOV coverage file")
    parser.add_argument(
        "--metrics-dir",
        required=True,
        help="Directory containing rust-code-analysis JSON metrics",
    )
    parser.add_argument("--clippy", required=True, help="Path to clippy JSON lines output")
    parser.add_argument("--machete", required=True, help="Path to cargo machete text output")
    parser.add_argument("--jscpd", required=True, help="Path to jscpd JSON report")
    parser.add_argument("--output-dir", required=True, help="Directory for summary outputs")
    return parser.parse_args()


def normalize_path(raw: str, repo_root: Path) -> str:
    candidate = Path(raw)
    if not candidate.is_absolute():
        candidate = repo_root / candidate
    resolved = candidate.resolve(strict=False)
    try:
        return resolved.relative_to(repo_root).as_posix()
    except ValueError:
        return resolved.as_posix()


def parse_lcov(path: Path, repo_root: Path) -> dict[str, dict[str, float]]:
    coverage: dict[str, dict[str, float]] = {}
    current_file: str | None = None
    lines_found = 0
    lines_hit = 0
    for raw_line in path.read_text().splitlines():
        line = raw_line.strip()
        if line.startswith("SF:"):
            current_file = normalize_path(line[3:], repo_root)
            lines_found = 0
            lines_hit = 0
        elif line.startswith("LF:"):
            lines_found = int(line[3:])
        elif line.startswith("LH:"):
            lines_hit = int(line[3:])
        elif line == "end_of_record" and current_file:
            ratio = (lines_hit / lines_found) if lines_found else 0.0
            coverage[current_file] = {
                "lines_found": lines_found,
                "lines_hit": lines_hit,
                "line_coverage": ratio,
            }
            current_file = None
    return coverage


def first_numeric(value: Any) -> float | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, dict):
        for key in ("sum", "value", "average", "total", "cyclomatic_complexity", "wmc", "mi"):
            if key in value:
                found = first_numeric(value[key])
                if found is not None:
                    return found
        for nested in value.values():
            found = first_numeric(nested)
            if found is not None:
                return found
    return None


def metric_value(container: dict[str, Any], *names: str) -> float | None:
    lowered = {name.lower() for name in names}
    for key, value in container.items():
        if key.lower() in lowered:
            found = first_numeric(value)
            if found is not None:
                return found
    return None


def parse_metrics_file(path: Path, metrics_root: Path, repo_root: Path) -> tuple[str, dict[str, float]]:
    payload = json.loads(path.read_text())
    metrics = payload.get("metrics", payload) if isinstance(payload, dict) else {}
    source_name = path.relative_to(metrics_root).as_posix()
    if source_name.endswith(".json"):
        source_name = source_name[:-5]
    return normalize_path(source_name, repo_root), {
        "wmc": metric_value(metrics, "wmc"),
        "cyclomatic_complexity": metric_value(metrics, "cc", "cyclomatic_complexity"),
        "cognitive_complexity": metric_value(metrics, "cognitive", "cognitive_complexity"),
        "maintainability_index": metric_value(metrics, "mi", "maintainability_index"),
    }


def parse_metrics_dir(path: Path, repo_root: Path) -> dict[str, dict[str, float]]:
    metrics: dict[str, dict[str, float]] = {}
    for json_file in sorted(path.rglob("*.json")):
        file_path, values = parse_metrics_file(json_file, path, repo_root)
        metrics[file_path] = values
    return metrics


def parse_clippy(path: Path, repo_root: Path) -> dict[str, Any]:
    findings = []
    code_counts: Counter[str] = Counter()
    for raw_line in path.read_text().splitlines():
        line = raw_line.strip()
        if not line.startswith("{"):
            continue
        payload = json.loads(line)
        message = payload.get("message", {})
        code = (message.get("code") or {}).get("code", "unknown")
        level = message.get("level")
        if level not in {"warning", "error"}:
            continue
        if code != "dead_code" and not code.startswith("unused"):
            continue
        spans = message.get("spans") or []
        file_name = spans[0].get("file_name") if spans else "<unknown>"
        findings.append(
            {
                "code": code,
                "level": level,
                "message": message.get("message", ""),
                "file": normalize_path(file_name, repo_root) if file_name != "<unknown>" else file_name,
            }
        )
        code_counts[code] += 1
    return {"count": len(findings), "codes": dict(code_counts), "findings": findings}


def parse_machete(path: Path) -> dict[str, Any]:
    lines = [line.strip() for line in path.read_text().splitlines() if line.strip()]
    return {"count": len(lines), "entries": lines}


def parse_jscpd(path: Path) -> dict[str, Any]:
    payload = json.loads(path.read_text())
    statistics = payload.get("statistics", {})
    total = statistics.get("total", {}) if isinstance(statistics, dict) else {}
    duplicates = payload.get("duplicates", [])
    return {
        "percentage": first_numeric(total.get("percentage", 0)) or 0.0,
        "lines": int(first_numeric(total.get("lines", 0)) or 0),
        "clones": int(first_numeric(total.get("clones", len(duplicates))) or len(duplicates)),
        "duplicates": duplicates[:25] if isinstance(duplicates, list) else [],
    }


def file_crap_score(complexity: float, coverage_ratio: float) -> float:
    return complexity**2 * (1 - coverage_ratio) ** 3 + complexity


def build_summary(
    coverage: dict[str, dict[str, float]],
    metrics: dict[str, dict[str, float]],
    clippy: dict[str, Any],
    machete: dict[str, Any],
    jscpd: dict[str, Any],
) -> dict[str, Any]:
    total_lines_found = sum(item["lines_found"] for item in coverage.values())
    total_lines_hit = sum(item["lines_hit"] for item in coverage.values())
    file_scores = []
    for file_name, metric_values in metrics.items():
        coverage_item = coverage.get(file_name)
        complexity = metric_values.get("wmc") or metric_values.get("cyclomatic_complexity")
        if coverage_item is None or complexity is None:
            continue
        file_scores.append(
            {
                "file": file_name,
                "crap": round(file_crap_score(complexity, coverage_item["line_coverage"]), 2),
                "complexity": round(complexity, 2),
                "line_coverage_percent": round(coverage_item["line_coverage"] * 100, 2),
                "maintainability_index": metric_values.get("maintainability_index"),
            }
        )
    file_scores.sort(key=lambda item: item["crap"], reverse=True)
    return {
        "coverage": {
            "files": len(coverage),
            "lines_found": total_lines_found,
            "lines_hit": total_lines_hit,
            "line_percent": round((total_lines_hit / total_lines_found) * 100, 2)
            if total_lines_found
            else 0.0,
        },
        "metrics": {
            "files": len(metrics),
            "with_complexity": sum(
                1
                for values in metrics.values()
                if values.get("wmc") is not None or values.get("cyclomatic_complexity") is not None
            ),
        },
        "clippy": clippy,
        "machete": machete,
        "jscpd": jscpd,
        "crap": {
            "model": "file-level CRAP using WMC or CC plus LCOV line coverage",
            "top_files": file_scores[:20],
        },
    }


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# Quality Summary",
        "",
        "## Coverage",
        "",
        f"- Files with coverage data: {summary['coverage']['files']}",
        f"- Line coverage: {summary['coverage']['line_percent']}% ({summary['coverage']['lines_hit']}/{summary['coverage']['lines_found']})",
        "",
        "## Dead code / unused findings",
        "",
        f"- Clippy dead/unused findings: {summary['clippy']['count']}",
        f"- cargo machete findings: {summary['machete']['count']}",
        f"- jscpd duplication: {summary['jscpd']['percentage']}% across {summary['jscpd']['clones']} clones",
        "",
        "## Approximate file-level CRAP hotspots",
        "",
        "| File | CRAP | Complexity | Line coverage |",
        "| --- | ---: | ---: | ---: |",
    ]
    for item in summary["crap"]["top_files"][:10]:
        lines.append(
            f"| `{item['file']}` | {item['crap']:.2f} | {item['complexity']:.2f} | {item['line_coverage_percent']:.2f}% |"
        )
    if not summary["crap"]["top_files"]:
        lines.append("| _No CRAP data available_ | | | |")
    lines.extend(
        [
            "",
            "## Notes",
            "",
            "- CRAP is approximate and file-level because Rust coverage tooling and static metrics are produced by separate tools.",
            "- Use the raw artifacts under `target/quality/` for deeper drill-down.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> None:
    args = parse_args()
    repo_root = Path(args.repo_root).resolve()
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    summary = build_summary(
        parse_lcov(Path(args.lcov), repo_root),
        parse_metrics_dir(Path(args.metrics_dir), repo_root),
        parse_clippy(Path(args.clippy), repo_root),
        parse_machete(Path(args.machete)),
        parse_jscpd(Path(args.jscpd)),
    )

    (output_dir / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    (output_dir / "summary.md").write_text(render_markdown(summary) + "\n")
    print(json.dumps(summary["coverage"]))


if __name__ == "__main__":
    main()
