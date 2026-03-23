#!/usr/bin/env python3
"""
Compute **d** = diff between LLVM LCOV and a static path-flow JSON (e.g. get_connector_with_networks.json).

Path-flow model (what we optimize vs what is context only)
==========================================================

- **Leaf (target)** — The **last** function in ``flows[].chain`` with ``role == \"target\"`` (or the
  artifact's root symbol). **Only this function's body** is scored against LCOV line hits. The goal
  is to **maximize how many leaf lines execute at least once** (minimize zero-hit / missing-DA lines
  inside that body).

- **Chain (everything before the leaf)** — Describes **how execution reaches** the leaf: HTTP
  endpoints, handler names, upstream callees, and branch hints. It is **not** given its own
  line-coverage budget in ``d``. When the leaf still has uncovered lines, you **walk the chain
  backward** and use endpoints / params / settings / branches to hypothesize new ways to hit
  those lines (that reasoning belongs in **specs**, not in the diff math).

This is **phase 1** only: produce ``d`` for a later **specs** step. It does **not** run any feedback loop.

Intended full loop (pseudo-code — mostly TODO elsewhere)::

    pl = []  # queue of executables (curl / tests / …)
    while not done:
        # run items from pl, collect profiles → lcov  (not implemented here)
        d = coverage_vs_flow_diff(lcov, flow_json)   # ← this script
        specs = f(d, audit_logs, context)            # TODO: spec generation
        pl += specs_to_pl(specs)                     # TODO
    # For now: pl stays empty, inner body is skipped; you only need **d**.

**Inputs:** any valid ``lcov.info`` (CI, grcov, local — however you produce it) + flow artifact.

Usage::

  python3 scripts/coverage_flow_gap.py \\
      --flow-json get_connector_with_networks.json \\
      --lcov lcov.info \\
      --repo-root . \\
      --out coverage_flow_gap.json

Options:
  --targets-only     Only report frames with role == \"target\" (leaf bodies).
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterator

# Shared semantics for JSON reports and spec context (imported by coverage_feedback_loop).
PATH_FLOW_MODEL: dict[str, str] = {
    "scored_for_coverage": "leaf_function_body_only",
    "leaf": (
        "Terminal function in the flow (chain step with role='target' / artifact root). "
        "LCOV line hits are evaluated only inside this function's body span."
    ),
    "chain": (
        "Earlier chain steps + endpoints are reachability context: which routes/handlers/branches "
        "lead into the leaf. Not scored separately; use when planning how to hit uncovered leaf lines."
    ),
    "objective": (
        "Maximize distinct leaf lines hit at least once; reduce gaps (zero-hit or missing DA) "
        "inside the leaf. Use chain metadata to vary params, config, or paths that affect "
        "branches on the way to the leaf."
    ),
}


def reachability_hints_from_artifact(flow_doc: dict[str, Any]) -> dict[str, Any]:
    """
    Lightweight view of **how** the leaf can be reached (for spec generation).

    Pulls top-level ``endpoints`` (method, path, handler, optional ``chain`` symbol list).
    Not used for LCOV scoring — only for human/LLM context when reducing leaf gaps.
    """
    eps = flow_doc.get("endpoints") or []
    out: list[dict[str, Any]] = []
    for e in eps:
        if not isinstance(e, dict):
            continue
        out.append(
            {
                "method": e.get("method"),
                "path": e.get("path"),
                "handler": e.get("handler"),
                "chain": e.get("chain"),
            }
        )
    return {"endpoint_count": len(out), "endpoints": out}


@dataclass(frozen=True)
class ChainFrame:
    """One step in flows[].chain."""

    flow_id: int
    function: str
    file: str
    def_line: int
    role: str
    source: str | None

    @property
    def dedupe_key(self) -> tuple[str, str, int]:
        return (self.file, self.function, self.def_line)


@dataclass
class BodySpan:
    start_line: int  # 1-based, absolute in repo file
    end_line: int


@dataclass
class LineGap:
    frame: ChainFrame
    span: BodySpan
    lines_in_span: int
    probed_lines: int
    hit_lines: int
    lines_without_lcov_da: int
    zero_hit_lines: list[int] = field(default_factory=list)

    @property
    def ratio(self) -> float | None:
        if self.probed_lines == 0:
            return None
        return self.hit_lines / self.probed_lines

    @property
    def status(self) -> str:
        if self.probed_lines == 0:
            return "no_lcov_da_in_span"
        if self.hit_lines == self.probed_lines:
            return "all_probed_lines_hit"
        if self.hit_lines == 0:
            return "all_probed_lines_zero"
        return "partial"


def note_no_lcov_da_in_span(span: BodySpan, hits: dict[int, int]) -> str:
    """Human hint when the leaf body span has no ``DA:`` rows but the file may still appear in lcov."""
    base = (
        "No DA: entries in lcov for this body span (feature/cfg mismatch vs instrumented "
        "binary, stale flow JSON line numbers, or LLVM omitted probes)."
    )
    if hits and any(ln < span.start_line or ln > span.end_line for ln in hits):
        base += (
            " This file has DA: lines outside this span — common causes: `async fn` lowering (probes "
            "not on the `async fn` lines), or an inlined leaf (use `#[inline(never)]` on a sync "
            "helper and point the path-flow leaf at it, e.g. `shallow_health_body`)."
        )
    return base


def _iter_chain_frames(flow_doc: dict[str, Any]) -> Iterator[ChainFrame]:
    flows = flow_doc.get("flows") or []
    for flow in flows:
        fid = int(flow.get("flow_id", -1))
        for step in flow.get("chain") or []:
            raw = step.get("source")
            if not isinstance(raw, str) or not raw.strip():
                raw = step.get("full_source")
            source = raw if isinstance(raw, str) and raw.strip() else None
            yield ChainFrame(
                flow_id=fid,
                function=str(step.get("function", "")),
                file=str(step.get("file", "")),
                def_line=int(step.get("def_line", 0)),
                role=str(step.get("role", "")),
                source=source,
            )


def _first_fn_body_span_lines(source: str, def_line: int) -> BodySpan | None:
    """
    Assume ``source`` starts at ``def_line`` in the real file. Return absolute [def_line, end]
    for the first top-level fn item using naive brace counting from the first ``{`` after ``fn``.
    """
    if not source or def_line < 1:
        return None
    m = re.search(r"\bfn\s+[A-Za-z0-9_]+", source)
    if not m:
        return None
    brace0 = source.find("{", m.end())
    if brace0 < 0:
        return None
    depth = 0
    close_pos = -1
    for i, c in enumerate(source[brace0:], start=brace0):
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                close_pos = i
                break
    if close_pos < 0:
        return None
    rel_start = source[:brace0].count("\n")  # 0-based offset from def_line
    rel_end = source[: close_pos + 1].count("\n")
    return BodySpan(start_line=def_line + rel_start, end_line=def_line + rel_end)


def parse_lcov_records(path: Path) -> dict[str, dict[int, int]]:
    """
    Map normalized relative path -> { line_no -> hit_count }.
    Only DA: lines are used (line coverage).
    """
    text = path.read_text(encoding="utf-8", errors="replace")
    by_file: dict[str, dict[int, int]] = {}
    current_sf: str | None = None
    for raw in text.splitlines():
        line = raw.strip()
        if line.startswith("SF:"):
            current_sf = line[3:].strip()
            if current_sf not in by_file:
                by_file[current_sf] = {}
        elif line.startswith("DA:") and current_sf:
            try:
                rest = line[3:]
                num, _, hits = rest.partition(",")
                ln = int(num)
                h = int(hits)
            except ValueError:
                continue
            prev = by_file[current_sf].get(ln, 0)
            by_file[current_sf][ln] = prev + h
        elif line == "end_of_record":
            current_sf = None
    return by_file


def normalize_sf_key(sf: str, repo_root: Path) -> str:
    """Turn absolute or mixed SF: paths into repo-relative forward-slash paths."""
    p = Path(sf)
    try:
        rel = p.resolve().relative_to(repo_root.resolve())
        return rel.as_posix()
    except ValueError:
        return sf.replace("\\", "/")


def build_normalized_lcov(
    by_file: dict[str, dict[int, int]], repo_root: Path
) -> dict[str, dict[int, int]]:
    out: dict[str, dict[int, int]] = defaultdict(dict)
    for sf, lines in by_file.items():
        key = normalize_sf_key(sf, repo_root)
        out[key].update(lines)
    return dict(out)


def gap_for_span(hits: dict[int, int], span: BodySpan) -> tuple[int, int, int, list[int], int]:
    """Returns (probed_lines, hit_lines, lines_in_span, zero_list, lines_without_da)."""
    zeros: list[int] = []
    probed = 0
    hit = 0
    lines_in_span = span.end_line - span.start_line + 1
    without_da = 0
    for ln in range(span.start_line, span.end_line + 1):
        if ln not in hits:
            without_da += 1
            continue
        probed += 1
        if hits[ln] > 0:
            hit += 1
        else:
            zeros.append(ln)
    return probed, hit, lines_in_span, zeros, without_da


def compute_d(
    flow_doc: dict[str, Any],
    lcov: dict[str, dict[int, int]],
    *,
    targets_only: bool,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """
    Build **d**: LCOV gaps for selected chain frames.

    With ``targets_only=True`` (recommended for the leaf-first model), **only leaf (target) bodies**
    appear in ``gaps``. Intermediate chain functions are excluded from scoring; they remain in the
    artifact for **spec / reachability** use only.
    ``lcov`` keys are repo-relative paths (forward slashes).
    """
    frames_by_key: dict[tuple[str, str, int], ChainFrame] = {}
    flow_ids_by_key: dict[tuple[str, str, int], set[int]] = defaultdict(set)

    for fr in _iter_chain_frames(flow_doc):
        if targets_only and fr.role != "target":
            continue
        if not fr.source:
            continue
        k = fr.dedupe_key
        flow_ids_by_key[k].add(fr.flow_id)
        if k not in frames_by_key:
            frames_by_key[k] = fr

    gap_dicts: list[dict[str, Any]] = []
    skipped: list[dict[str, Any]] = []

    for _k, fr in sorted(frames_by_key.items(), key=lambda x: (x[1].file, x[1].def_line)):
        span = _first_fn_body_span_lines(fr.source or "", fr.def_line)
        if span is None:
            skipped.append(
                {
                    "function": fr.function,
                    "file": fr.file,
                    "def_line": fr.def_line,
                    "reason": "could_not_parse_body_span",
                }
            )
            continue
        file_key = fr.file.replace("\\", "/")
        hits = lcov.get(file_key, {})
        if not hits:
            skipped.append(
                {
                    "function": fr.function,
                    "file": fr.file,
                    "def_line": fr.def_line,
                    "reason": "no_lcov_for_file",
                }
            )
            continue

        probed, hit, lines_in_span, zeros, without_da = gap_for_span(hits, span)
        lg = LineGap(
            frame=fr,
            span=span,
            lines_in_span=lines_in_span,
            probed_lines=probed,
            hit_lines=hit,
            lines_without_lcov_da=without_da,
            zero_hit_lines=zeros,
        )
        gap_dicts.append(
            {
                "function": lg.frame.function,
                "file": lg.frame.file,
                "role": lg.frame.role,
                "def_line": lg.frame.def_line,
                "body_span": {"start": lg.span.start_line, "end": lg.span.end_line},
                "lines_in_span": lg.lines_in_span,
                "lines_without_lcov_da": lg.lines_without_lcov_da,
                "lcov_probed_lines": lg.probed_lines,
                "lcov_hit_lines": lg.hit_lines,
                "line_coverage_ratio": lg.ratio,
                "zero_hit_lines": lg.zero_hit_lines,
                "status": lg.status,
                "note": (
                    note_no_lcov_da_in_span(lg.span, hits)
                    if lg.status == "no_lcov_da_in_span"
                    else None
                ),
            }
        )

    return gap_dicts, skipped


def extract_leaf_from_chain_artifact(flow_doc: dict[str, Any]) -> dict[str, Any] | None:
    """
    Build **LEAF** from chain artifact: first ``flows[].chain`` step with ``role == \"target\"``
    and a usable ``source`` / ``full_source`` (same rules as ``_iter_chain_frames``).
    """
    for fr in _iter_chain_frames(flow_doc):
        if fr.role == "target" and fr.source:
            return {
                "name": fr.function,
                "file": fr.file,
                "def_line": fr.def_line,
                "source": fr.source,
            }
    return None


def compute_leaf_gap(
    leaf: dict[str, Any],
    lcov: dict[str, dict[int, int]],
) -> tuple[dict[str, Any] | None, str | None]:
    """
    **d** for a single leaf only (uncovered vs ``lcov`` inside body span).
    Returns ``(gap_dict, None)`` or ``(None, skip_reason)``.
    """
    name = str(leaf.get("name", ""))
    file = str(leaf.get("file", ""))
    def_line = int(leaf.get("def_line", 0))
    source = leaf.get("source")
    if not isinstance(source, str) or not source.strip():
        return None, "leaf_missing_source"
    span = _first_fn_body_span_lines(source, def_line)
    if span is None:
        return None, "could_not_parse_body_span"
    file_key = file.replace("\\", "/")
    hits = lcov.get(file_key, {})
    if not hits:
        return None, "no_lcov_for_file"
    probed, hit, lines_in_span, zeros, without_da = gap_for_span(hits, span)
    lg = LineGap(
        frame=ChainFrame(
            flow_id=-1,
            function=name,
            file=file,
            def_line=def_line,
            role="target",
            source=source,
        ),
        span=span,
        lines_in_span=lines_in_span,
        probed_lines=probed,
        hit_lines=hit,
        lines_without_lcov_da=without_da,
        zero_hit_lines=zeros,
    )
    note = note_no_lcov_da_in_span(lg.span, hits) if lg.status == "no_lcov_da_in_span" else None
    return (
        {
            "function": lg.frame.function,
            "file": lg.frame.file,
            "role": "target",
            "def_line": lg.frame.def_line,
            "body_span": {"start": lg.span.start_line, "end": lg.span.end_line},
            "lines_in_span": lg.lines_in_span,
            "lines_without_lcov_da": lg.lines_without_lcov_da,
            "lcov_probed_lines": lg.probed_lines,
            "lcov_hit_lines": lg.hit_lines,
            "line_coverage_ratio": lg.ratio,
            "zero_hit_lines": lg.zero_hit_lines,
            "status": lg.status,
            "note": note,
        },
        None,
    )


def main() -> int:
    ap = argparse.ArgumentParser(description="Flow JSON vs LCOV gap report (POC).")
    ap.add_argument(
        "--flow-json",
        type=Path,
        required=True,
        help="Static flow artifact (e.g. get_connector_with_networks.json).",
    )
    ap.add_argument(
        "--lcov",
        type=Path,
        default=Path("lcov.info"),
        help="lcov.info from grcov (default: ./lcov.info).",
    )
    ap.add_argument(
        "--repo-root",
        type=Path,
        default=Path("."),
        help="Repository root (for normalizing SF: paths).",
    )
    ap.add_argument(
        "--out",
        type=Path,
        default=None,
        help="Write JSON report to this path (default: stdout only).",
    )
    ap.add_argument(
        "--targets-only",
        action="store_true",
        help=(
            "Only score the leaf (role == 'target'). Matches path-flow model: chain is context, "
            "only the terminal function body is in gaps."
        ),
    )
    args = ap.parse_args()
    repo_root = args.repo_root.resolve()

    if not args.flow_json.is_file():
        print(f"Missing flow JSON: {args.flow_json}", file=sys.stderr)
        return 1
    if not args.lcov.is_file():
        print(f"Missing lcov: {args.lcov}", file=sys.stderr)
        return 1

    flow_doc = json.loads(args.flow_json.read_text(encoding="utf-8"))
    raw_lcov = parse_lcov_records(args.lcov)
    lcov = build_normalized_lcov(raw_lcov, repo_root)

    gaps, skipped = compute_d(flow_doc, lcov, targets_only=args.targets_only)

    d: dict[str, Any] = {
        "kind": "coverage_vs_static_flow",
        "root_function": flow_doc.get("function"),
        "flow_json": str(args.flow_json.resolve()),
        "lcov": str(args.lcov.resolve()),
        "repo_root": str(repo_root),
        "targets_only": args.targets_only,
        "frames_analyzed": len(gaps),
        "frames_skipped": skipped,
        "gaps": gaps,
    }

    report: dict[str, Any] = {
        "path_flow_model": PATH_FLOW_MODEL,
        "feedback_loop": {
            "stage": "diff_only",
            "pl": [],
            "inner_loop_ran": False,
            "description": (
                "pl is empty and specs_to_pl is not implemented; only d is produced for later spec generation."
            ),
            "todo": {
                "collect_lcov": "Run pl items, merge profiles → lcov (runner not in this script).",
                "specs": "specs = f(d, audit_logs, context)",
                "extend_pl": "pl += specs_to_pl(specs)",
                "iterate": "Repeat until coverage gap small / stagnation / budget.",
            },
        },
        "d": d,
    }

    text = json.dumps(report, indent=2)
    print(text)
    if args.out:
        args.out.write_text(text + "\n", encoding="utf-8")
        print(f"\nWrote {args.out}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
