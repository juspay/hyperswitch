#!/usr/bin/env python3
"""
Single-pass: optional **pl** (executables) → **leaf vs LCOV diff (d)** from chain artifact + ``lcov.info``.

**Cypress:** There is no spec named after ``get_connector_with_networks`` (internal Rust). Payment
E2E under ``cypress-tests/`` calls **v1** ``/payments`` APIs (see ``cypress/support/commands.js``),
which overlap the **reachability** endpoints in ``get_connector_with_networks.json``. Whether the
**leaf** body runs depends on runtime (e.g. debit routing + connector ``network``); generic payment
flows may still improve LCOV for ``payments.rs`` if the build matches.

**pl-json:** See ``scripts/coverage_pl_cypress.example.json``. Use ``--allow-exec`` to actually run
``subprocess`` items. **After** Cypress against an **instrumented** router, regenerate ``lcov.info``
(``just coverage_html``) before expecting ``d`` to change.

- **Explanation** — TODO; see ``explanation`` in output.

Path-flow model: ``coverage_flow_gap.PATH_FLOW_MODEL``.

Usage::

  python3 scripts/coverage_feedback_loop.py \\
      --chain-artifact get_connector_with_networks.json \\
      --lcov lcov.info \\
      --repo-root . \\
      --out coverage_run_report.json

  # Optional: run example Cypress pl (needs live API + cypress-tests deps)
  python3 scripts/coverage_feedback_loop.py \\
      --pl-json scripts/coverage_pl_cypress.example.json \\
      --allow-exec \\
      --chain-artifact get_connector_with_networks.json \\
      --lcov lcov.info

Optional ``--audit-json`` is accepted and echoed in the report for a future explanation step.

By default a **human-readable diff** is printed to **stderr** before JSON on stdout. Use ``--json-only`` for JSON only.
Use ``--print-line-hits`` to print each line in the leaf body span with LCOV hit counts on stderr (works with ``--json-only``).
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))

import coverage_flow_gap as cfg  # noqa: E402


def load_chain_artifact(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def build_leaf_from_artifact(flow_doc: dict[str, Any]) -> dict[str, Any] | None:
    leaf = cfg.extract_leaf_from_chain_artifact(flow_doc)
    if leaf is None:
        return None
    return {
        "name": leaf["name"],
        "file": leaf["file"],
        "def_line": leaf["def_line"],
        "source": leaf["source"],
    }


def load_audit_json(path: Path | None) -> Any:
    if path is None or not path.is_file():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def load_pl_json(path: Path) -> list[Any]:
    raw = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(raw, dict) and "items" in raw:
        return list(raw["items"])
    if isinstance(raw, list):
        return raw
    raise ValueError("pl-json must be a JSON array or {\"items\": [...]}")


def run_pl_item(
    item: Any,
    *,
    index: int,
    repo_root: Path,
    allow_exec: bool,
) -> dict[str, Any]:
    rec: dict[str, Any] = {"index": index, "item": item, "status": "unknown"}
    if not isinstance(item, dict):
        rec["status"] = "skipped_non_object"
        return rec
    rec["id"] = item.get("id")
    kind = str(item.get("kind", "noop")).lower()

    if kind in ("noop", "echo", "log"):
        rec["status"] = "noop"
        return rec

    if kind != "subprocess":
        rec["status"] = "skipped_unknown_kind"
        return rec

    argv = item.get("argv")
    if not allow_exec:
        rec["status"] = "skipped_no_allow_exec"
        rec["hint"] = "Pass --allow-exec to run subprocess items"
        return rec
    if not isinstance(argv, list) or not argv or not all(isinstance(x, str) for x in argv):
        rec["status"] = "error_bad_argv"
        return rec

    cwd_raw = item.get("cwd")
    if cwd_raw:
        cwd = (repo_root / str(cwd_raw)).resolve()
    else:
        cwd = repo_root
    timeout = float(item.get("timeout", 600))
    try:
        cp = subprocess.run(
            argv,
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
        rec["status"] = "subprocess_done"
        rec["cwd"] = str(cwd)
        rec["returncode"] = cp.returncode
        rec["stdout_tail"] = (cp.stdout or "")[-1500:]
        rec["stderr_tail"] = (cp.stderr or "")[-1500:]
    except subprocess.TimeoutExpired as e:
        rec["status"] = "subprocess_timeout"
        rec["error"] = str(e)
    except OSError as e:
        rec["status"] = "subprocess_os_error"
        rec["error"] = str(e)
    return rec


def load_lcov_as_profile(lcov_path: Path, repo_root: Path) -> dict[str, dict[int, int]]:
    raw = cfg.parse_lcov_records(lcov_path)
    return cfg.build_normalized_lcov(raw, repo_root)


def static_regions_for_leaf(LEAF: dict[str, Any]) -> cfg.BodySpan | None:
    return cfg._first_fn_body_span_lines(str(LEAF.get("source", "")), int(LEAF.get("def_line", 0)))


def gap_uncovered(LEAF: dict[str, Any], llvm_profile: dict[str, dict[int, int]]) -> dict[str, Any]:
    gap, err = cfg.compute_leaf_gap(LEAF, llvm_profile)
    return {
        "kind": "leaf_uncovered_lines",
        "leaf": {"name": LEAF["name"], "file": LEAF["file"], "def_line": LEAF["def_line"]},
        "gaps": [gap] if gap is not None else [],
        "error": err,
    }


EXPLANATION_TODO: dict[str, Any] = {
    "status": "todo",
    "message": (
        "Implement explanation: combine d (leaf gap), audit_trail, and context to describe "
        "what executed / what did not and why."
    ),
    "inputs_ready": ["d", "context", "audit_trail"],
}


def print_pathflow_vs_llvm_diff(
    d: dict[str, Any],
    *,
    lcov_path: Path,
    endpoint_count: int,
    file: Any = sys.stderr,
) -> None:
    """Readable console summary: path-flow leaf span vs LLVM LCOV hits."""
    leaf = d.get("leaf") or {}
    name = leaf.get("name", "?")
    path = leaf.get("file", "?")
    def_line = leaf.get("def_line", "?")

    lines = [
        "",
        "=" * 78,
        "PATH-FLOW GRAPH (leaf)  vs  LLVM / GRCOV (lcov.info)",
        "=" * 78,
        f"  Leaf (from chain artifact):  {name}",
        f"  File:                         {path}",
        f"  Symbol def_line (artifact):   {def_line}",
        f"  LCOV file:                    {lcov_path.resolve()}",
        "",
        "  Path-flow side: body line span is derived from the leaf source snippet in the JSON.",
        "  LLVM side:      DA: line hit counts from lcov for that file + span.",
        "-" * 78,
    ]

    if d.get("error"):
        lines.append(f"  DIFF ERROR: {d['error']} (could not compare)")
    elif not d.get("gaps"):
        lines.append("  DIFF: no gap row (unexpected empty gaps).")
    else:
        g = d["gaps"][0]
        bs = g.get("body_span") or {}
        lines.extend(
            [
                f"  Body span (lines):            {bs.get('start')} – {bs.get('end')} "
                f"({g.get('lines_in_span')} lines)",
                f"  LCOV lines with DA in span:    {g.get('lcov_probed_lines')}",
                f"  LCOV lines with hits > 0:      {g.get('lcov_hit_lines')}",
                f"  Lines in span but no DA:       {g.get('lines_without_lcov_da')}",
                f"  Zero-hit lines (if probed):    {g.get('zero_hit_lines') or []}",
                f"  Line coverage ratio:           {g.get('line_coverage_ratio')}",
                f"  Status:                        {g.get('status')}",
            ]
        )
        if g.get("note"):
            lines.append(f"  Note:                          {g.get('note')}")
    lines.extend(
        [
            "-" * 78,
            f"  Reachability (path-flow only): {endpoint_count} HTTP endpoint(s) list this leaf on their chain.",
            "  (Endpoints are context — not line-scored in d.)",
            "=" * 78,
            "",
        ]
    )
    print("\n".join(lines), file=file)


def print_leaf_line_hits(
    LEAF: dict[str, Any],
    llvm_profile: dict[str, dict[int, int]],
    repo_root: Path,
    *,
    file: Any = sys.stderr,
) -> None:
    """Per-line DA-style report for the leaf body span (source read from repo for context)."""
    span = static_regions_for_leaf(LEAF)
    if span is None:
        print("\n--- Per-line LCOV hits ---\n  (could not derive body span from leaf source)\n", file=file)
        return
    file_rel = str(LEAF["file"]).replace("\\", "/")
    hits = llvm_profile.get(file_rel, {})
    src_path = repo_root / file_rel
    lines_content: list[str] = []
    if src_path.is_file():
        lines_content = src_path.read_text(encoding="utf-8", errors="replace").splitlines()
    print("\n--- Per-line LCOV hits (leaf body span) ---", file=file)
    print(f"  {'line':>5}  {'hits':>12}  source", file=file)
    print("  " + "-" * 74, file=file)
    for ln in range(span.start_line, span.end_line + 1):
        if ln not in hits:
            hit_s = "(no DA)"
        else:
            hit_s = str(hits[ln])
        snip = (
            lines_content[ln - 1].rstrip()
            if 0 < ln <= len(lines_content)
            else ""
        )
        print(f"  {ln:5d}  {hit_s:>12}  {snip}", file=file)
    print("  " + "-" * 74, file=file)
    print("", file=file)


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Chain artifact + LCOV → leaf diff (d). pl empty; explanation TODO.",
    )
    ap.add_argument("--chain-artifact", type=Path, required=True)
    ap.add_argument("--lcov", type=Path, default=Path("lcov.info"))
    ap.add_argument("--repo-root", type=Path, default=Path("."))
    ap.add_argument(
        "--audit-json",
        type=Path,
        default=None,
        help="Optional; stored in report for future explanation step",
    )
    ap.add_argument("--out", type=Path, default=None)
    ap.add_argument(
        "--json-only",
        action="store_true",
        help="Suppress human-readable diff on stderr; print only JSON.",
    )
    ap.add_argument(
        "--pl-json",
        type=Path,
        default=None,
        help="Optional {\"items\":[...]} of executables (e.g. scripts/coverage_pl_cypress.example.json)",
    )
    ap.add_argument(
        "--allow-exec",
        action="store_true",
        help="Actually run kind=subprocess pl items (otherwise they are skipped safely)",
    )
    ap.add_argument(
        "--print-line-hits",
        action="store_true",
        help="After the summary, print each line in the leaf body span with LCOV hit counts (stderr).",
    )
    args = ap.parse_args()
    repo_root = args.repo_root.resolve()

    if not args.chain_artifact.is_file():
        print(f"Missing chain artifact: {args.chain_artifact}", file=sys.stderr)
        return 1
    if not args.lcov.is_file():
        print(f"Missing lcov: {args.lcov}", file=sys.stderr)
        return 1

    CHAIN_DOC = load_chain_artifact(args.chain_artifact)
    LEAF = build_leaf_from_artifact(CHAIN_DOC)
    if LEAF is None:
        print("Could not extract LEAF (target + source) from chain artifact.", file=sys.stderr)
        return 1

    pl: list[Any] = []
    run_records: list[dict[str, Any]] = []
    if args.pl_json is not None:
        if not args.pl_json.is_file():
            print(f"Missing pl-json: {args.pl_json}", file=sys.stderr)
            return 1
        pl = load_pl_json(args.pl_json)
        print(f"[coverage_feedback_loop] running pl: {len(pl)} item(s), allow_exec={args.allow_exec}", file=sys.stderr)
        for i, item in enumerate(pl):
            run_records.append(run_pl_item(item, index=i, repo_root=repo_root, allow_exec=args.allow_exec))
    else:
        print("[coverage_feedback_loop] no --pl-json; pl is empty (lcov read as-is).", file=sys.stderr)

    llvm_profile = load_lcov_as_profile(args.lcov, repo_root)
    span = static_regions_for_leaf(LEAF)
    d = gap_uncovered(LEAF, llvm_profile)

    audit_trail = load_audit_json(args.audit_json)

    context = {
        "path_flow_model": cfg.PATH_FLOW_MODEL,
        "leaf": {k: v for k, v in LEAF.items() if k != "source"},
        "reachability": cfg.reachability_hints_from_artifact(CHAIN_DOC),
        "chain_artifact_path": str(args.chain_artifact.resolve()),
        "leaf_line_span": (
            {"start": span.start_line, "end": span.end_line} if span else None
        ),
        "pl": pl,
        "pl_note": (
            "Regenerate lcov.info (e.g. just coverage_html) after instrumented runs if pl executed."
            if pl
            else "Pass --pl-json to run Cypress/curl steps before diff; or keep empty and use existing lcov."
        ),
    }

    pipeline = "diff_with_pl" if pl else "diff_only_pl_empty"
    final: dict[str, Any] = {
        "path_flow_model": cfg.PATH_FLOW_MODEL,
        "pipeline": pipeline,
        "pl": pl,
        "run_records": run_records,
        "lcov_path": str(args.lcov.resolve()),
        "d": d,
        "context": context,
        "audit_trail": audit_trail,
        "explanation": EXPLANATION_TODO,
        "LEAF_public": {k: v for k, v in LEAF.items() if k != "source"},
        "CHAIN_ARTIFACT": str(args.chain_artifact.resolve()),
    }

    reach = context.get("reachability") or {}
    ep_count = int(reach.get("endpoint_count") or 0)
    if not args.json_only:
        print_pathflow_vs_llvm_diff(d, lcov_path=args.lcov, endpoint_count=ep_count, file=sys.stderr)
    if args.print_line_hits:
        print_leaf_line_hits(LEAF, llvm_profile, repo_root, file=sys.stderr)

    text = json.dumps(final, indent=2, default=str)
    print(text)
    if args.out:
        args.out.write_text(text + "\n", encoding="utf-8")
        print(f"Wrote {args.out}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
