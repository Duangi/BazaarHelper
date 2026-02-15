#!/usr/bin/env python3
from __future__ import annotations

import json
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RES = ROOT / "src-tauri" / "resources"
OUT_JSON = RES / "resource_inventory.json"
OUT_MD = RES / "RESOURCE_REPORT.md"


def classify_name(name: str) -> str:
    if " " in name:
        return "contains-space"
    if any(ch.isupper() for ch in name) and any(ch.islower() for ch in name):
        return "mixed-case"
    if name.lower() == name:
        return "lowercase"
    if name.upper() == name:
        return "uppercase"
    return "other"


def main() -> None:
    if not RES.exists():
        raise SystemExit(f"resources path not found: {RES}")

    folders: Counter[str] = Counter()
    exts: Counter[str] = Counter()
    casing: Counter[str] = Counter()
    by_folder_ext: dict[str, Counter[str]] = defaultdict(Counter)

    for file_path in RES.rglob("*"):
        if not file_path.is_file():
            continue
        rel = file_path.relative_to(RES)
        folder = str(rel.parent)
        ext = file_path.suffix.lower() or "(none)"
        name = file_path.stem

        folders[folder] += 1
        exts[ext] += 1
        casing[classify_name(name)] += 1
        by_folder_ext[folder][ext] += 1

    payload = {
        "total_files": sum(folders.values()),
        "folders": dict(sorted(folders.items())),
        "extensions": dict(sorted(exts.items())),
        "name_style": dict(sorted(casing.items())),
        "folder_extensions": {k: dict(sorted(v.items())) for k, v in sorted(by_folder_ext.items())},
    }

    OUT_JSON.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")

    lines = [
        "# Resource Inventory",
        "",
        f"Total files: **{payload['total_files']}**",
        "",
        "## Folder counts",
    ]
    for folder, count in sorted(folders.items(), key=lambda x: (-x[1], x[0])):
        lines.append(f"- `{folder}`: {count}")

    lines += ["", "## Extension counts"]
    for ext, count in sorted(exts.items(), key=lambda x: (-x[1], x[0])):
        lines.append(f"- `{ext}`: {count}")

    lines += ["", "## Name style"]
    for style, count in sorted(casing.items(), key=lambda x: (-x[1], x[0])):
        lines.append(f"- `{style}`: {count}")

    OUT_MD.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {OUT_JSON}")
    print(f"wrote {OUT_MD}")


if __name__ == "__main__":
    main()
