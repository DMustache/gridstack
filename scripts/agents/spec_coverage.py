#!/usr/bin/env python3
"""Compare Matrix Client-Server OpenAPI operations against implemented axum routes.

Output: generated/reports/spec-coverage.md and generated/reports/spec-coverage.json
Exit code is 0 even when coverage is incomplete; the report is the contract.
"""
import json
import pathlib
import re
import sys
from urllib.parse import urlparse

ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = ROOT / "generated/openapi/matrix-openapi.json"
OUT_DIR = ROOT / "generated/reports"
HTTP_METHODS = ("get", "put", "post", "delete", "patch")


def normalize(path: str) -> str:
    return re.sub(r"\{[^}]+\}", "{}", path.rstrip("/"))


def server_prefix(servers) -> str:
    if not servers:
        return ""
    url = servers[0].get("url", "")
    url = re.sub(r"\{[^}]+\}", "", url)
    return urlparse(url).path.rstrip("/") if "://" in url else url.rstrip("/")


def spec_operations(spec: dict):
    top_prefix = server_prefix(spec.get("servers")) or spec.get("basePath", "").rstrip("/")
    for path, item in spec.get("paths", {}).items():
        path_prefix = server_prefix(item.get("servers")) or top_prefix
        for method in HTTP_METHODS:
            operation = item.get(method)
            if not operation:
                continue
            prefix = server_prefix(operation.get("servers")) or path_prefix
            full = path if path.startswith("/_matrix") else prefix + path
            tag = (operation.get("tags") or ["untagged"])[0]
            yield {
                "method": method.upper(),
                "path": full,
                "key": f"{method.upper()} {normalize(full)}",
                "tag": tag,
                "operation_id": operation.get("operationId", ""),
                "deprecated": bool(operation.get("deprecated")),
            }


ROUTE_PATTERN = re.compile(r'\.route\(\s*"(/(?:_matrix|\.well-known)/[^"]+)"\s*,(.*?)\)\s*(?=\.route|\.layer|\.with_state|\.merge|;|\n\s*\n)', re.S)
METHOD_PATTERN = re.compile(r"\b(get|put|post|delete|patch)\s*\(")


def implemented_routes():
    found = {}
    for source in (ROOT / "src").rglob("*.rs"):
        if "bin" in source.parts:
            continue
        text = source.read_text(encoding="utf-8")
        for match in ROUTE_PATTERN.finditer(text):
            path, handler_chain = match.group(1), match.group(2)
            for method in METHOD_PATTERN.findall(handler_chain):
                key = f"{method.upper()} {normalize(path)}"
                found[key] = str(source.relative_to(ROOT))
    return found


def main() -> int:
    if not SPEC.exists():
        print(f"missing {SPEC.relative_to(ROOT)}; run task 'Matrix: Sync & Generate OpenAPI'", file=sys.stderr)
        return 2
    spec = json.loads(SPEC.read_text(encoding="utf-8"))
    operations = list(spec_operations(spec))
    implemented = implemented_routes()
    spec_keys = {op["key"] for op in operations}

    by_tag = {}
    for op in operations:
        op["implemented_in"] = implemented.get(op["key"])
        by_tag.setdefault(op["tag"], []).append(op)

    active = [op for op in operations if not op["deprecated"]]
    done = [op for op in active if op["implemented_in"]]
    extra = sorted(key for key in implemented if key not in spec_keys)
    percent = (100.0 * len(done) / len(active)) if active else 0.0

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    lines = [
        "# Matrix Client-Server spec coverage",
        "",
        f"Non-deprecated operations implemented: **{len(done)}/{len(active)} ({percent:.1f}%)**",
        "",
        "Route presence only. Behavioral conformance is tracked by tests and task acceptance criteria.",
        "",
        "| Tag | Done | Total |",
        "| --- | ---: | ---: |",
    ]
    for tag in sorted(by_tag):
        tag_active = [op for op in by_tag[tag] if not op["deprecated"]]
        tag_done = [op for op in tag_active if op["implemented_in"]]
        lines.append(f"| {tag} | {len(tag_done)} | {len(tag_active)} |")
    lines += ["", "## Missing operations", ""]
    for tag in sorted(by_tag):
        missing = [op for op in by_tag[tag] if not op["implemented_in"] and not op["deprecated"]]
        if missing:
            lines.append(f"### {tag}")
            lines += [f"- `{op['method']} {op['path']}`" for op in missing]
            lines.append("")
    if extra:
        lines += ["## Implemented routes not found in spec (check path/version)", ""]
        lines += [f"- `{key}`" for key in extra]
    (OUT_DIR / "spec-coverage.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (OUT_DIR / "spec-coverage.json").write_text(
        json.dumps({"implemented": len(done), "total": len(active), "percent": round(percent, 1),
                    "operations": operations, "unmatched_routes": extra}, indent=2),
        encoding="utf-8",
    )
    print(f"spec coverage {len(done)}/{len(active)} ({percent:.1f}%)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
