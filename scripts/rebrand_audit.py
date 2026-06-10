#!/usr/bin/env python3
"""Sakrylle deep-rebrand audit.

Scans codex-rs/ and codex-cli/ for user-visible brand residue
(Codex / OpenAI / ChatGPT), classifies each occurrence, and buckets it
per docs/superpowers/specs/2026-06-10-sakrylle-cli-deep-rebrand-design.md.

Output: a JSON + a human-readable report of candidate occurrences.

This is a *candidate* list. Per-occurrence judgment still required (esp. bucket C).
Heuristics intentionally err toward INCLUDING borderline cases (flagged) rather
than silently dropping them.
"""
from __future__ import annotations

import json
import re
import sys
from dataclasses import dataclass, asdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET_DIRS = ["codex-rs", "codex-cli"]

BRAND_RE = re.compile(r"Codex|OpenAI|ChatGPT|chatgpt\.com|openai/codex|@openai/codex")

# Exclusions (paths)
def is_excluded_path(p: Path) -> bool:
    parts = p.parts
    name = p.name
    if name.endswith("_tests.rs") or name == "tests.rs":
        return True
    if "snapshots" in parts:
        return True
    if "/tests/" in str(p) or any(part == "tests" for part in parts):
        return True
    if name in ("Cargo.toml", "Cargo.lock"):
        return True
    if "node_modules" in parts or "target" in parts:
        return True
    return False

# Preserved env / config keys (do NOT touch) — substrings
PRESERVED_KEYS = [
    "CODEX_HOME", "CODEX_API_KEY", "OPENAI_API_KEY", "CODEX_INTERNAL_ORIGINATOR_OVERRIDE",
    "CODEX_SANDBOX", "CODEX_ACCESS_TOKEN", "CODEX_MANAGED", "OPENAI_BASE_URL",
    "OPENAI_ORGANIZATION", "OPENAI_PROJECT", "CODEX_API_BASE_URL", "OPENAI_DEFAULT_MODEL",
    "CODEX_RS", "CODEX_DISABLE", "CODEX_OSS", "CODEX_CACHE", "CODEX_CONFIG",
]

# agent-identity / protocol issuer constants (do NOT touch)
ISSUER_HINTS = ["codex-backend", "backend-api", "auth.openai.com", "/codex-backend/"]

# Internal-symbol patterns (identifiers, not user-visible text)
SYMBOL_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:codex|Codex|ChatGpt|chat_gpt|chatgpt)[A-Za-z0-9_]*")

# crude string-literal extraction for .rs / .js / .ts
STRING_RE = re.compile(r'"((?:[^"\\]|\\.)*)"')

@dataclass
class Occurrence:
    file: str
    line: int
    bucket: str          # A / B / C / SKIP
    reason: str
    in_string: bool
    text: str

def classify_line(path: Path, lineno: int, line: str) -> list[Occurrence]:
    out: list[Occurrence] = []
    stripped = line.strip()
    is_comment = stripped.startswith("//") or stripped.startswith("*") or stripped.startswith("/*")

    strings = [m.group(1) for m in STRING_RE.finditer(line)]
    brand_in_string = any(BRAND_RE.search(s) for s in strings)

    # preserved env/config keys
    if any(k in line for k in PRESERVED_KEYS) and not brand_in_string_excluding_keys(strings):
        # the brand match is only within a preserved key
        if not _brand_outside_preserved(line):
            return []

    rel = str(path.relative_to(ROOT))

    # comment → SKIP (attribution comments retained) — check BEFORE URL classification
    if is_comment:
        out.append(Occurrence(rel, lineno, "SKIP", "comment (attribution retained)", brand_in_string, stripped[:200]))
        return out

    # internal routing host-checks (upstream-compat logic like uses_codex_backend) → SKIP
    if re.search(r'starts_with\(\s*"https://chatgpt\.com', line) or re.search(r'==\s*"chatgpt\.com"', line) \
            or "ends_with(\".chatgpt.com\")" in line or 'chatgpt_hosts' in rel \
            or re.search(r'host\s*==\s*"chatgpt\.com"', line):
        out.append(Occurrence(rel, lineno, "SKIP", "internal host-routing check (upstream-compat logic)", brand_in_string, stripped[:200]))
        return out

    # URL buckets
    if "chatgpt.com" in line or "openai/codex" in line or "@openai/codex" in line:
        # issuer / protocol constants are SKIP
        if any(h in line for h in ISSUER_HINTS):
            out.append(Occurrence(rel, lineno, "SKIP", "issuer/protocol-or-backend-api constant", brand_in_string, stripped[:200]))
            return out
        if "chatgpt.com/backend-api" in line or "chatgpt.com/device" in line or "chatgpt.com/#settings" in line:
            out.append(Occurrence(rel, lineno, "C", "functional endpoint (chatgpt.com path)", brand_in_string, stripped[:200]))
            return out
        if "github.com/openai/codex" in line or "@openai/codex" in line:
            out.append(Occurrence(rel, lineno, "B", "upstream repo / npm attribution URL", brand_in_string, stripped[:200]))
            return out
        out.append(Occurrence(rel, lineno, "C", "chatgpt.com reference (verify reachability)", brand_in_string, stripped[:200]))
        return out

    # brand only inside a string literal → classify cosmetic vs protocol-value
    if brand_in_string:
        # HTTP header names / wire values — MUST NOT CHANGE (protocol)
        if re.search(r'"[^"]*OpenAI-[A-Za-z]', line) or re.search(r'"X-OpenAI-', line) \
                or "OpenAI-Beta" in line or "OpenAI-Organization" in line or "OpenAI-Project" in line:
            out.append(Occurrence(rel, lineno, "SKIP", "HTTP header / wire value (protocol, must not change)", True, stripped[:200]))
            return out
        # filesystem path components for upstream config dirs — MUST NOT CHANGE (config loading)
        if re.search(r'(join|Path::new)\(\s*"(OpenAI|Codex)"', line):
            out.append(Occurrence(rel, lineno, "SKIP", "upstream FS path component (config loading, verify before touch)", True, stripped[:200]))
            return out
        # provider id / display-name constants — MUST NOT CHANGE (provider routing)
        if re.search(r'(PROVIDER_NAME|provider_id|"openai")', line) and "OpenAI" in line:
            out.append(Occurrence(rel, lineno, "SKIP", "provider id/name (routing, must not change)", True, stripped[:200]))
            return out
        out.append(Occurrence(rel, lineno, "A", "user-visible string literal", True, stripped[:200]))
        return out

    # brand present but only as an identifier/symbol → SKIP (internal symbol)
    out.append(Occurrence(rel, lineno, "SKIP", "identifier/symbol (not user-visible)", False, stripped[:200]))
    return out

def brand_in_string_excluding_keys(strings: list[str]) -> bool:
    for s in strings:
        if BRAND_RE.search(s) and not any(k in s for k in PRESERVED_KEYS):
            return True
    return False

def _brand_outside_preserved(line: str) -> bool:
    # Replace preserved keys, then see if brand still matches
    tmp = line
    for k in PRESERVED_KEYS:
        tmp = tmp.replace(k, "")
    return bool(BRAND_RE.search(tmp))

def test_line_ranges(text: str) -> set[int]:
    """Return 1-based line numbers that fall inside a `#[cfg(test)]` block.

    Tracks brace depth starting from the line after a `#[cfg(test)]` attribute so
    inline test modules in non-`_tests.rs` files are excluded like real test files.
    """
    lines = text.splitlines()
    in_test = set()
    i = 0
    n = len(lines)
    while i < n:
        if "#[cfg(test)]" in lines[i]:
            # find the opening brace of the following item, then balance braces
            depth = 0
            started = False
            j = i
            while j < n:
                for ch in lines[j]:
                    if ch == "{":
                        depth += 1
                        started = True
                    elif ch == "}":
                        depth -= 1
                if started:
                    in_test.add(j + 1)  # 1-based
                    if depth == 0:
                        break
                else:
                    in_test.add(j + 1)
                j += 1
            i = j + 1
            continue
        i += 1
    return in_test


def main() -> int:
    occs: list[Occurrence] = []
    for d in TARGET_DIRS:
        base = ROOT / d
        for ext in ("*.rs", "*.js", "*.ts", "*.mjs", "*.cjs"):
            for path in base.rglob(ext):
                if is_excluded_path(path):
                    continue
                try:
                    text = path.read_text(encoding="utf-8", errors="replace")
                except Exception:
                    continue
                test_lines = test_line_ranges(text)
                for i, line in enumerate(text.splitlines(), start=1):
                    if BRAND_RE.search(line):
                        if i in test_lines:
                            occs.append(Occurrence(str(path.relative_to(ROOT)), i, "SKIP", "inline test code", False, line.strip()[:200]))
                            continue
                        occs.extend(classify_line(path, i, line))

    buckets: dict[str, list[Occurrence]] = {"A": [], "B": [], "C": [], "SKIP": []}
    for o in occs:
        buckets[o.bucket].append(o)

    summary = {k: len(v) for k, v in buckets.items()}
    files_actionable = sorted({o.file for o in occs if o.bucket in ("A", "B", "C")})

    out = {
        "summary": summary,
        "actionable_files": len(files_actionable),
        "buckets": {k: [asdict(o) for o in v] for k, v in buckets.items()},
    }
    Path(ROOT / "scripts" / "rebrand_audit.json").write_text(json.dumps(out, indent=2))

    # human report
    print(f"=== SUMMARY === {summary}  actionable_files={len(files_actionable)}")
    for bk in ("A", "B", "C"):
        print(f"\n===== BUCKET {bk} ({len(buckets[bk])}) =====")
        byfile: dict[str, list[Occurrence]] = {}
        for o in buckets[bk]:
            byfile.setdefault(o.file, []).append(o)
        for f in sorted(byfile):
            print(f"  {f}  ({len(byfile[f])})")
    return 0

if __name__ == "__main__":
    sys.exit(main())
