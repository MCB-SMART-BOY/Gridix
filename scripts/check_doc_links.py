#!/usr/bin/env python3
"""
Validate local Markdown links in README.md and docs/*.md.

Usage:
  python scripts/check_doc_links.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")


def iter_docs(root: Path) -> list[Path]:
    files = [root / "README.md"]
    files.extend(sorted((root / "docs").glob("*.md")))
    return [p for p in files if p.exists()]


def is_external(link: str) -> bool:
    return link.startswith("http://") or link.startswith("https://") or link.startswith("#")


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    broken: list[tuple[Path, str]] = []

    for doc in iter_docs(root):
        content = doc.read_text(encoding="utf-8")
        for match in LINK_RE.finditer(content):
            link = match.group(1).strip()
            if not link or is_external(link):
                continue
            target = (doc.parent / link).resolve()
            if not target.exists():
                broken.append((doc.relative_to(root), link))

    if broken:
        print("Broken local links found:")
        for source, link in broken:
            print(f"  - {source} -> {link}")
        return 1

    print("OK: no local broken links")
    return 0


if __name__ == "__main__":
    sys.exit(main())
