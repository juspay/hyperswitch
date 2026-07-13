#!/usr/bin/env python3
"""
track_feature_history.py — Identify when features were introduced across git tags.

Scans the last N daily git tags (format: 2026.04.21.0), extracts feature sets
at each tag by running extract_features.py, and shows which features are new
in each tag compared to the previous one.

Usage:
  python3 scripts/track_feature_history.py [--tags 10] [--csv feature_history.csv]

Output:
  - feature_history.csv  (repo root)
  - feature_introductions table in features.db
  - tag_snapshots table in features.db  (full snapshot per tag)
  - Summary table printed to stdout
"""

import os
import sys
import csv
import re
import shutil
import subprocess
import tempfile
import argparse
import sqlite3

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DB_PATH = os.path.join(REPO_ROOT, "features.db")
EXTRACT_SCRIPT = os.path.join(REPO_ROOT, "scripts", "extract_features.py")

# Tags matching YYYY.MM.DD.N exactly (no hotfix suffix)
TAG_PATTERN = re.compile(r'^\d{4}\.\d{2}\.\d{2}\.\d+$')

# ---------------------------------------------------------------------------
# B3 dynamic detection — parse Diesel struct fields from Rust source.
# The hardcoded BUCKET_3_FEATURES list in extract_features.py is the same
# for every tag (since we copy the current script into each checkout), so
# it produces 0 diffs. These helpers read the actual Rust files at each
# tag so a new business_profile field → new B3 feature.
# ---------------------------------------------------------------------------

B3_SOURCES = [
    # (rust_file_relative_path, struct_name, source_label)
    ("crates/diesel_models/src/business_profile.rs", "Profile", "business_profile"),
    ("crates/diesel_models/src/merchant_account.rs", "MerchantAccount", "merchant_account"),
]

# Fields that are identifiers / timestamps / internal plumbing — not features.
B3_SKIP_FIELDS = {
    "id", "created_at", "modified_at", "deleted", "version",
    "profile_id", "merchant_id", "organization_id", "profile_name",
    "merchant_name", "merchant_details", "locker_id", "publishable_key",
    "storage_scheme", "primary_business_details", "recon_status",
    "payment_link_config_id", "payout_link_config_id", "metadata",
    "payment_response_hash_key", "routing_algorithm_id",
    "payout_routing_algorithm_id",
}


def extract_struct_fields(text, struct_name):
    """
    Walk a Rust file and return the list of `pub <field>:` names declared in
    every `pub struct <struct_name> { ... }` block. Handles v1/v2 cfg-gated
    duplicate structs by collecting fields from all matches (union happens
    naturally when added to a set).
    """
    fields = []
    lines = text.splitlines()
    header_re = re.compile(rf'pub\s+struct\s+{re.escape(struct_name)}\b')
    field_re = re.compile(r'^\s*pub\s+(\w+)\s*:')

    i = 0
    n = len(lines)
    while i < n:
        if header_re.search(lines[i]):
            while i < n and '{' not in lines[i]:
                i += 1
            if i >= n:
                break
            depth = lines[i].count('{') - lines[i].count('}')
            i += 1
            while i < n and depth > 0:
                m = field_re.match(lines[i])
                if m:
                    fields.append(m.group(1))
                depth += lines[i].count('{') - lines[i].count('}')
                i += 1
            continue
        i += 1
    return fields


def detect_b3_fields(worktree_path):
    """
    Return a set of (bucket=3, '', '', '', 'source.field') tuples by scanning
    the Diesel struct definitions actually present in this tag's codebase.
    """
    keys = set()
    for rel_path, struct_name, label in B3_SOURCES:
        abs_path = os.path.join(worktree_path, rel_path)
        if not os.path.exists(abs_path):
            continue
        try:
            with open(abs_path, "r", encoding="utf-8") as f:
                text = f.read()
        except OSError:
            continue
        for field in extract_struct_fields(text, struct_name):
            if field in B3_SKIP_FIELDS:
                continue
            keys.add((3, "", "", "", f"{label}.{field}"))
    return keys


def run(cmd, cwd=None, check=True):
    result = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if check and result.returncode != 0:
        print(f"  WARN [{' '.join(str(c) for c in cmd)}] exit={result.returncode}", file=sys.stderr)
        if result.stderr:
            print(f"  {result.stderr.strip()[:300]}", file=sys.stderr)
    return result.stdout.strip()


def get_tags(n):
    """Return last N main daily tags (YYYY.MM.DD.N, no suffix), sorted oldest→newest."""
    raw = run(["git", "tag", "-l", "--sort=version:refname", "20*"])
    tags = [t.strip() for t in raw.splitlines() if TAG_PATTERN.match(t.strip())]
    return tags[-n:]


def collect_features(worktree_path):
    """
    Run extract_features.py inside a worktree checkout and return two
    frozensets of (bucket, connector, pm, pmt, feature) tuples:

      all_keys     — every feature present in the codebase at this tag
      covered_keys — subset where cypress_test_status == 'covered'

    B3 features come from dynamic struct parsing (see detect_b3_fields);
    their cypress status is not tracked because the hardcoded B3 list
    would produce the same answer for every tag.
    """
    dest_script = os.path.join(worktree_path, "scripts", "extract_features.py")
    os.makedirs(os.path.dirname(dest_script), exist_ok=True)
    shutil.copy2(EXTRACT_SCRIPT, dest_script)

    subprocess.run(
        [sys.executable, dest_script],
        cwd=worktree_path,
        capture_output=True,
        text=True,
    )

    all_keys = set()
    covered_keys = set()

    def slurp(csv_path, keyfn):
        if not os.path.exists(csv_path):
            return
        with open(csv_path, newline="", encoding="utf-8") as f:
            for row in csv.DictReader(f):
                key = keyfn(row)
                all_keys.add(key)
                if row.get("cypress_test_status") == "covered":
                    covered_keys.add(key)

    slurp(
        os.path.join(worktree_path, "bucket_1_connector_features.csv"),
        lambda r: (1, r.get("connector", ""), "", "", r.get("feature", "")),
    )
    slurp(
        os.path.join(worktree_path, "bucket_2_connector_pm_features.csv"),
        lambda r: (
            2,
            r.get("connector", ""),
            r.get("payment_method", ""),
            r.get("payment_method_type", ""),
            r.get("feature", ""),
        ),
    )
    # Bucket 3 is read from bucket_3_core_features.csv (the hardcoded core
    # feature list in extract_features.py). This makes B3 counts and cypress
    # coverage % match the feature_extraction_report numbers. B3 introductions
    # over time will be ~flat because the list is identical across tags.
    slurp(
        os.path.join(worktree_path, "bucket_3_core_features.csv"),
        lambda r: (3, "", "", "", r.get("feature", "")),
    )

    return frozenset(all_keys), frozenset(covered_keys)


def setup_db(conn):
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS tag_snapshots (
            tag        TEXT    NOT NULL,
            bucket     INTEGER NOT NULL,
            connector  TEXT    NOT NULL DEFAULT '',
            pm         TEXT    NOT NULL DEFAULT '',
            pmt        TEXT    NOT NULL DEFAULT '',
            feature    TEXT    NOT NULL,
            is_covered INTEGER NOT NULL DEFAULT 0,
            is_blocked_by_bug INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (tag, bucket, connector, pm, pmt, feature)
        );

        CREATE TABLE IF NOT EXISTS feature_introductions (
            bucket            INTEGER NOT NULL,
            connector         TEXT    NOT NULL DEFAULT '',
            pm                TEXT    NOT NULL DEFAULT '',
            pmt               TEXT    NOT NULL DEFAULT '',
            feature           TEXT    NOT NULL,
            introduced_in_tag TEXT    NOT NULL,
            PRIMARY KEY (bucket, connector, pm, pmt, feature)
        );

        CREATE TABLE IF NOT EXISTS cypress_test_introductions (
            bucket            INTEGER NOT NULL,
            connector         TEXT    NOT NULL DEFAULT '',
            pm                TEXT    NOT NULL DEFAULT '',
            pmt               TEXT    NOT NULL DEFAULT '',
            feature           TEXT    NOT NULL,
            covered_in_tag    TEXT    NOT NULL,
            PRIMARY KEY (bucket, connector, pm, pmt, feature)
        );
    """)
    # Idempotent migration: add is_covered column for DBs created before it
    try:
        conn.execute("ALTER TABLE tag_snapshots ADD COLUMN is_covered INTEGER NOT NULL DEFAULT 0")
    except sqlite3.OperationalError:
        pass
    # Idempotent migration: add is_blocked_by_bug column for DBs created before it
    try:
        conn.execute("ALTER TABLE tag_snapshots ADD COLUMN is_blocked_by_bug INTEGER NOT NULL DEFAULT 0")
    except sqlite3.OperationalError:
        pass
    conn.commit()


def main():
    parser = argparse.ArgumentParser(
        description="Track feature introduction across daily git tags"
    )
    parser.add_argument(
        "--tags", type=int, default=10,
        help="Number of recent daily tags to scan (default: 10)",
    )
    parser.add_argument(
        "--csv", default="feature_history.csv",
        help="Output CSV filename, written to repo root (default: feature_history.csv)",
    )
    parser.add_argument(
        "--head-tag", default=None, metavar="TAG",
        help=(
            "Snapshot the current HEAD under TAG and append it as the final entry "
            "(e.g. --head-tag 2026.05.13.0). Use when the tag hasn't been cut yet "
            "but you want to preview coverage at HEAD."
        ),
    )
    args = parser.parse_args()

    # If --head-tag was requested, create a lightweight local tag so get_tags()
    # picks it up normally, then clean it up after the run.
    _head_tag_created = False
    if args.head_tag:
        existing = run(["git", "tag", "-l", args.head_tag]).strip()
        if not existing:
            run(["git", "tag", args.head_tag, "HEAD"])
            _head_tag_created = True
            print(f"  Tagged HEAD as {args.head_tag} (temporary)", file=sys.stderr)
        else:
            print(f"  Tag {args.head_tag} already exists, using it as-is", file=sys.stderr)

    try:
        tags = get_tags(args.tags)
        if not tags:
            print("ERROR: No daily tags found matching YYYY.MM.DD.N pattern.", file=sys.stderr)
            sys.exit(1)

        # Ensure the requested head tag is the final entry even if --tags N
        # happened to exclude it due to count.
        if args.head_tag and args.head_tag not in tags:
            tags.append(args.head_tag)

        print(f"Scanning {len(tags)} tags: {tags[0]}  →  {tags[-1]}", file=sys.stderr)
        print(f"  (baseline = {tags[0]}, new features compared against previous tag)", file=sys.stderr)

        worktree = tempfile.mkdtemp(prefix="hs_hist_")
        tag_features: dict[str, frozenset] = {}
        tag_covered: dict[str, frozenset] = {}

        try:
            run(["git", "worktree", "add", "--detach", worktree, tags[0]])

            for i, tag in enumerate(tags):
                print(f"  [{i+1:2d}/{len(tags)}] {tag} ...", file=sys.stderr, end="", flush=True)
                if i > 0:
                    run(["git", "-C", worktree, "checkout", "--detach", tag])
                feats, covered = collect_features(worktree)
                tag_features[tag] = feats
                tag_covered[tag] = covered
                print(f"  {len(feats):5d} features, {len(covered):5d} cypress-covered", file=sys.stderr)

        finally:
            run(["git", "worktree", "remove", "--force", worktree], check=False)
            shutil.rmtree(worktree, ignore_errors=True)

    finally:
        # Always remove the temporary tag regardless of success or failure
        if _head_tag_created:
            run(["git", "tag", "-d", args.head_tag], check=False)
            print(f"  Removed temporary tag {args.head_tag}", file=sys.stderr)

    # ------------------------------------------------------------------ #
    # Feature introductions: in tag N but not tag N-1                      #
    # Cypress test introductions: covered in tag N but not in tag N-1     #
    # ------------------------------------------------------------------ #
    new_per_tag: dict[str, list] = {tag: [] for tag in tags}
    new_covered_per_tag: dict[str, list] = {tag: [] for tag in tags}
    for i, tag in enumerate(tags[1:], start=1):
        prev = tags[i - 1]
        new_per_tag[tag] = sorted(tag_features[tag] - tag_features[prev])
        new_covered_per_tag[tag] = sorted(tag_covered[tag] - tag_covered[prev])

    removed_per_tag: dict[str, list] = {tag: [] for tag in tags}
    for i, tag in enumerate(tags[1:], start=1):
        prev = tags[i - 1]
        removed_per_tag[tag] = sorted(tag_features[prev] - tag_features[tag])

    # ------------------------------------------------------------------ #
    # Persist to DB                                                         #
    # ------------------------------------------------------------------ #
    conn = sqlite3.connect(DB_PATH)
    setup_db(conn)

    # Pre-load blocked_by_bug status from issues table for efficient lookup
    cursor = conn.execute("SELECT bucket, connector, pm, pmt, feature, blocked_by_bug FROM issues")
    blocked_map = {
        (b, c or '', pm or '', pmt or '', f): blk
        for b, c, pm, pmt, f, blk in cursor.fetchall()
    }

    for tag, feats in tag_features.items():
        covered = tag_covered[tag]

        # Save manual overrides before DELETE so they survive re-scan.
        # A "manual override" is any row where is_covered=1 or is_blocked_by_bug=1
        # that the spec-walk would NOT have set (i.e., not in the auto-covered set
        # and not in the blocked_map).  These represent human decisions that must
        # persist across re-scans.
        overrides = {}
        for b, c, pm, pmt, f, ic, blk in conn.execute(
            "SELECT bucket, connector, pm, pmt, feature, is_covered, is_blocked_by_bug "
            "FROM tag_snapshots WHERE tag = ?", (tag,)
        ):
            key = (b, c or '', pm or '', pmt or '', f)
            auto_covered = key in covered
            auto_blocked = blocked_map.get(key, 0) == 1
            if (ic == 1 and not auto_covered) or (blk == 1 and not auto_blocked):
                overrides[key] = (ic, blk)

        conn.execute("DELETE FROM tag_snapshots WHERE tag = ?", (tag,))
        conn.executemany(
            "INSERT OR REPLACE INTO tag_snapshots VALUES (?,?,?,?,?,?,?,?)",
            [
                (
                    tag,
                    f[0],
                    f[1],
                    f[2],
                    f[3],
                    f[4],
                    1 if f in covered else overrides.get((f[0], f[1] or '', f[2] or '', f[3] or '', f[4]), (0, 0))[0],
                    0  # is_blocked_by_bug: forced to 0 when covered (invariant: covered items cannot be bug-blocked)
                    if (f in covered or overrides.get((f[0], f[1] or '', f[2] or '', f[3] or '', f[4]), (0, 0))[0])
                    else max(
                        blocked_map.get((f[0], f[1] or '', f[2] or '', f[3] or '', f[4]), 0),
                        overrides.get((f[0], f[1] or '', f[2] or '', f[3] or '', f[4]), (0, 0))[1],
                    ),
                )
                for f in feats
            ],
        )
    conn.commit()

    conn.execute("DELETE FROM feature_introductions")
    conn.executemany(
        "INSERT OR REPLACE INTO feature_introductions VALUES (?,?,?,?,?,?)",
        [(k[0], k[1], k[2], k[3], k[4], tag)
         for tag in tags[1:] for k in new_per_tag[tag]],
    )

    conn.execute("DELETE FROM cypress_test_introductions")
    conn.executemany(
        "INSERT OR REPLACE INTO cypress_test_introductions VALUES (?,?,?,?,?,?)",
        [(k[0], k[1], k[2], k[3], k[4], tag)
         for tag in tags[1:] for k in new_covered_per_tag[tag]],
    )
    conn.commit()
    conn.close()

    # ------------------------------------------------------------------ #
    # Sync issues table from the latest tag's tag_snapshots               #
    #                                                                      #
    # extract_features.py runs its spec-walk against the CURRENT working  #
    # tree, so when a cypress spec file lands on main but the working      #
    # branch hasn't merged it yet, issues.cypress_status stays             #
    # 'not_covered' even though the tag worktree (scanned above) detected  #
    # it as covered.  Syncing here closes that gap automatically after     #
    # every track_feature_history run.                                     #
    # ------------------------------------------------------------------ #
    latest_tag = tags[-1]
    conn = sqlite3.connect(DB_PATH)
    synced = conn.execute("""
        UPDATE issues
        SET cypress_status = CASE WHEN ts.is_covered = 1 THEN 'covered' ELSE 'not_covered' END,
            status = CASE
                WHEN ts.is_covered = 1                          THEN 'covered'
                WHEN issues.status = 'covered' AND ts.is_covered = 0 THEN 'open'
                ELSE issues.status
            END,
            updated_at = CURRENT_TIMESTAMP
        FROM tag_snapshots ts
        WHERE ts.tag        = ?
          AND ts.bucket     = issues.bucket
          AND ts.connector  = issues.connector
          AND ts.pm         = issues.pm
          AND ts.pmt        = issues.pmt
          AND ts.feature    = issues.feature
          AND ts.is_covered != (CASE WHEN issues.cypress_status = 'covered' THEN 1 ELSE 0 END)
    """, (latest_tag,)).rowcount
    conn.commit()
    conn.close()
    if synced:
        print(f"  Synced {synced} issues row(s) from latest tag snapshot ({latest_tag})",
              file=sys.stderr)

    # ------------------------------------------------------------------ #
    # CSVs                                                                  #
    # ------------------------------------------------------------------ #
    out_csv = os.path.join(REPO_ROOT, args.csv)
    with open(out_csv, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["introduced_in_tag", "bucket", "connector", "pm", "pmt", "feature"])
        for tag in tags[1:]:
            for k in new_per_tag[tag]:
                w.writerow([tag, k[0], k[1], k[2], k[3], k[4]])

    cypress_csv = os.path.join(REPO_ROOT, "cypress_test_history.csv")
    with open(cypress_csv, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["covered_in_tag", "bucket", "connector", "pm", "pmt", "feature"])
        for tag in tags[1:]:
            for k in new_covered_per_tag[tag]:
                w.writerow([tag, k[0], k[1], k[2], k[3], k[4]])

    # ------------------------------------------------------------------ #
    # Summary                                                               #
    # ------------------------------------------------------------------ #
    print()
    header = f"{'Tag':<22} {'+B1':>5} {'+B2':>5} {'+B3':>5} {'+Total':>7} {'+Cypress':>10} {'Removed':>8}"
    print(header)
    print("-" * len(header))

    tb1 = tb2 = tb3 = ta = tc = 0
    for i, tag in enumerate(tags):
        keys = new_per_tag[tag]
        b1 = sum(1 for k in keys if k[0] == 1)
        b2 = sum(1 for k in keys if k[0] == 2)
        b3 = sum(1 for k in keys if k[0] == 3)
        cc = len(new_covered_per_tag[tag])
        rm = len(removed_per_tag[tag])
        note = "  ← baseline" if i == 0 else ""
        print(f"{tag:<22} {b1:>5} {b2:>5} {b3:>5} {len(keys):>7} {cc:>10} {rm:>8}{note}")
        tb1 += b1; tb2 += b2; tb3 += b3; ta += len(keys); tc += cc

    print("-" * len(header))
    print(f"{'TOTAL new (excl. baseline)':<22} {tb1:>5} {tb2:>5} {tb3:>5} {ta:>7} {tc:>10}")
    print()
    print(f"Feature CSV: {out_csv}")
    print(f"Cypress CSV: {cypress_csv}")
    print(f"DB:          {DB_PATH}")
    print(f"  Tables: tag_snapshots, feature_introductions, cypress_test_introductions")
    print()
    print("Sample queries:")
    print(f"  sqlite3 {DB_PATH} \"SELECT covered_in_tag, COUNT(*) FROM cypress_test_introductions GROUP BY covered_in_tag ORDER BY covered_in_tag;\"")
    print(f"  sqlite3 {DB_PATH} \"SELECT tag, SUM(is_covered) AS covered, COUNT(*) AS total FROM tag_snapshots GROUP BY tag ORDER BY tag DESC LIMIT 10;\"")


if __name__ == "__main__":
    main()
