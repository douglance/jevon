#!/usr/bin/env python3
"""Extract public Rust items with their documentation, for `jev classify`.

Emits a JSON array of item texts on stdout, and a parallel array of locations
on the path given by --locations. The two are index-aligned, because `classify`
reports answers in the order it was given items and carries nothing else back.

    audits/extract-items.py --locations /tmp/loc.json > /tmp/items.json
    jev classify --questions-file audits/rust-canon.json \
                 --items-file /tmp/items.json --json
"""

import argparse
import json
import pathlib
import re
import sys

# A public item worth judging. Private helpers are excluded deliberately: the
# questions are about what a caller is held to, and a caller cannot see them.
ITEM = re.compile(
    r"^\s*pub(?:\s*\(\s*crate\s*\))?\s+"
    r"(?:async\s+|const\s+|unsafe\s+|extern\s+\"[^\"]*\"\s+)*"
    r"(fn|struct|enum|trait|type|const|static)\s+([A-Za-z_][A-Za-z0-9_]*)"
)
DOC = re.compile(r"^\s*///\s?(.*)$")
ATTR = re.compile(r"^\s*#\[")


def signature(lines, start):
    """The item's signature, from its first line to the `{` or `;` that ends it."""
    out = []
    depth = 0
    for line in lines[start : start + 20]:
        out.append(line.rstrip())
        depth += line.count("(") - line.count(")")
        if depth <= 0 and (line.rstrip().endswith("{") or line.rstrip().endswith(";")):
            break
    return "\n".join(out)


def items_in(path, root):
    lines = path.read_text(encoding="utf-8").splitlines()
    found = []
    doc, doc_start = [], None

    for i, line in enumerate(lines):
        matched_doc = DOC.match(line)
        if matched_doc:
            if doc_start is None:
                doc_start = i
            doc.append(matched_doc.group(1))
            continue
        if ATTR.match(line) and doc:
            continue  # attributes may sit between the doc and the item
        matched_item = ITEM.match(line)
        if matched_item:
            found.append(
                {
                    "kind": matched_item.group(1),
                    "name": matched_item.group(2),
                    "doc": "\n".join(doc).strip(),
                    "signature": signature(lines, i),
                    "file": str(path.relative_to(root)),
                    "line": i + 1,
                }
            )
        doc, doc_start = [], None

    return found


def as_text(item):
    """What the model sees. The file path is omitted on purpose: it names the
    crate, and a crate called `cmd-doctor` would nudge every answer about an
    item inside it before the item itself was read."""
    doc = item["doc"] or "(no documentation)"
    return f"/// {doc}\n{item['signature']}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--locations", required=True)
    parser.add_argument("--limit", type=int, default=0)
    args = parser.parse_args()

    root = pathlib.Path(args.root).resolve()
    found = []
    for path in sorted(root.glob("crates/*/src/**/*.rs")):
        found.extend(items_in(path, root))
    found.sort(key=lambda i: (i["file"], i["line"]))
    if args.limit:
        found = found[: args.limit]

    pathlib.Path(args.locations).write_text(json.dumps(found, indent=1))
    json.dump([as_text(i) for i in found], sys.stdout)
    print(f"{len(found)} items", file=sys.stderr)


if __name__ == "__main__":
    main()
