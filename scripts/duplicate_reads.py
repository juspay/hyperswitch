#!/usr/bin/env python3
"""Aggregate duplicate record reads per API session, from router logs.

The router finds these itself: every database call passes through
`track_database_call` (crates/diesel_models/src/query/generics.rs), which checks
the statement against the session's read ledger, and the session reports what it
read twice as one WARN line:

    records read more than once in this API session
        duplicate_reads=3 records=2
        duplicates=[{"table":"payment_intent","repeats":2,
                     "first_read_from":"...get_trackers@payments.rs:519",
                     "re_read_from":["...call_connector@payments.rs:2030", ...]}]

This script rolls those lines up per flow, for a sweep or to compare a run
against a budget. It detects nothing itself. WARN is enough, so the log it needs
is small.

Given logs from a build without the ledger it falls back to inferring
duplicates from the statement lines the funnel logs at DEBUG (`query` plus
`span_path`): the same statement, same binds, twice in one session. Same
meaning; the reported numbers are what the service actually acted on.

Usage:
    duplicate_reads.py logs/debug.log.*
    duplicate_reads.py logs/debug.log.* --json report.json
    duplicate_reads.py logs/debug.log.* --baseline budget.json --update-baseline
    duplicate_reads.py logs/debug.log.* --baseline budget.json
    duplicate_reads.py --self-test

Records must be JSON. The file sink always is, whatever the console is set to:
    ROUTER__LOG__FILE__ENABLED=true   # writes logs/debug.log.<hour>
"""

import argparse
import collections
import json
import re
import sys

# `debug_query` emits the statement followed by its binds:
#   SELECT ... FROM "payment_intent" WHERE ... -- binds: [pay_123, merchant_1]
BINDS_RE = re.compile(r"\s*--\s*binds:\s*\[.*\]\s*$", re.DOTALL)
WS_RE = re.compile(r"\s+")

TABLE_RES = (
    ("select", re.compile(r'\bFROM\s+"([^"]+)"', re.IGNORECASE)),
    ("insert", re.compile(r'\bINSERT\s+INTO\s+"([^"]+)"', re.IGNORECASE)),
    ("update", re.compile(r'\bUPDATE\s+"([^"]+)"', re.IGNORECASE)),
    ("delete", re.compile(r'\bDELETE\s+FROM\s+"([^"]+)"', re.IGNORECASE)),
)

READS = ("select", "count")
WRITES = ("insert", "update", "delete")


def classify_sql(sql):
    """Return (operation, table) for a debug_query statement."""
    head = sql.lstrip()[:32].upper()
    if head.startswith("INSERT"):
        op = "insert"
    elif head.startswith("UPDATE"):
        op = "update"
    elif head.startswith("DELETE"):
        op = "delete"
    elif head.startswith("SELECT"):
        op = "count" if re.match(r"SELECT\s+COUNT\(", head) else "select"
    else:
        op = "other"

    # DELETE/UPDATE also contain FROM in subqueries, so match the writer's own
    # shape first and only fall back to the generic FROM.
    for name, regex in TABLE_RES:
        if name != op:
            continue
        found = regex.search(sql)
        if found:
            return op, found.group(1)
    found = TABLE_RES[0][1].search(sql)
    return op, found.group(1) if found else "unknown"


def identity(sql):
    """Key for 'this record, via this lookup' -- SQL and binds together.

    The statement itself, whitespace-normalised. Not a hash of it: a shared key
    would count two records as repeats of each other, and there is nothing to
    gain here from risking that.
    """
    return WS_RE.sub(" ", sql).strip()


SELECT_COLS_RE = re.compile(r"^SELECT\s+(.*?)\s+FROM\s", re.IGNORECASE | re.DOTALL)
# Projections wider than this are collapsed to a count; narrow ones stay legible.
WIDE_PROJECTION = 6


def redact(sql, show_binds):
    """Make a statement readable: the table, the predicate, the binds.

    A projected column list runs to dozens of columns and identifies nothing,
    so it collapses to a count. Binds stay out unless asked for: a bound
    statement can carry merchant data into an artifact.
    """
    sql = WS_RE.sub(" ", sql).strip()
    if not show_binds:
        sql = BINDS_RE.sub(" -- binds: [redacted]", sql)
    columns = SELECT_COLS_RE.match(sql)
    if columns:
        count = columns.group(1).count(",") + 1
        if count > WIDE_PROJECTION:
            sql = f"{sql[: columns.start(1)]}<{count} columns>{sql[columns.end(1) :]}"
    return sql


def field(record, name):
    """Read a log field, from the top level or from `extra`.

    Custom event fields land under `extra` when the router is built with
    `log_custom_entries_to_extra` (it is); the formatter's own keys stay at the
    top level. Check both rather than depend on the build.
    """
    value = record.get(name)
    if value is None:
        extra = record.get("extra")
        if isinstance(extra, dict):
            value = extra.get(name)
    return value


def extract_reported(record):
    """One session's duplicate reads, as the router reported them, or None.

    The line carries the whole list -- table, repeat count and call sites per
    record -- as JSON in its `duplicates` field, so nothing is re-derived here.
    """
    detail = field(record, "duplicates")
    if not isinstance(detail, str):
        return None
    try:
        entries = json.loads(detail)
    except (ValueError, TypeError):
        return None
    if not isinstance(entries, list):
        return None
    request_id = record.get("request_id")
    if not isinstance(request_id, str) or not request_id:
        return None

    duplicates = []
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        repeats = entry.get("repeats")
        if not isinstance(repeats, int) or repeats < 1:
            continue
        origins = entry.get("re_read_from")
        duplicates.append(
            {
                "table": str(entry.get("table") or "unknown"),
                "repeats": repeats,
                # A read the span tree could not place is reported as unknown
                # rather than dropped: the repeat still happened.
                "first_read_from": entry.get("first_read_from") or "?",
                "re_read_from": [
                    origin or "?" for origin in (origins if isinstance(origins, list) else [])
                ],
            }
        )
    if not duplicates:
        return None

    flow = record.get("flow") or record.get("request_url_path") or "UNKNOWN"
    return {"request_id": request_id, "flow": str(flow), "duplicates": duplicates}


def extract(record):
    """Pull a query line out of a parsed log record as (sql, request_id, flow).

    `request_id` comes back None when the line carries a statement but no
    correlation -- a distinct failure mode worth reporting rather than silently
    dropping, since it means the root span never stamped it.
    """
    query = field(record, "query")
    if not isinstance(query, str) or not query.strip():
        return None
    # `span_path` is the root-to-leaf chain of `#[instrument]` span names, so it
    # names the call site. `fn` (the innermost span alone) is the fallback when
    # the chain is a single span, or for logs predating it.
    where = field(record, "span_path") or record.get("fn") or "?"
    request_id = record.get("request_id")
    if not isinstance(request_id, str) or not request_id:
        return query, str(where), None, None
    flow = record.get("flow") or record.get("request_url_path") or "UNKNOWN"
    return query, str(where), request_id, str(flow)


def parse_log(lines):
    """Group query lines into per-session ordered read/write sequences."""
    sessions = collections.OrderedDict()
    reported = []
    seen_lines = 0
    uncorrelated = 0
    for line in lines:
        line = line.strip()
        if not line or not line.startswith("{"):
            continue
        try:
            record = json.loads(line)
        except (ValueError, TypeError):
            continue
        if not isinstance(record, dict):
            continue
        duplicate = extract_reported(record)
        if duplicate is not None:
            reported.append(duplicate)
            continue

        found = extract(record)
        if found is None:
            continue
        sql, where, request_id, flow = found
        if request_id is None:
            uncorrelated += 1
            continue
        seen_lines += 1
        op, table = classify_sql(sql)
        session = sessions.setdefault(request_id, {"flow": flow, "calls": []})
        # `flow` is recorded onto the span mid-request, so later lines carry the
        # real name while the first few may still say UNKNOWN.
        if session["flow"] == "UNKNOWN" and flow != "UNKNOWN":
            session["flow"] = flow
        session["calls"].append((op, table, sql, where))
    return sessions, reported, seen_lines, uncorrelated


def analyse_session(calls):
    """Group each record's reads, in order, with where each was issued.

    A statement's read list holds the original followed by every repeat, so the
    report can name the call site that fetched it first and those that fetched
    it again. Writes are skipped: only a read repeats a read.
    """
    occurrences = collections.OrderedDict()
    tables_read = collections.Counter()

    for seq, (op, table, sql, where) in enumerate(calls):
        if op not in READS:
            continue
        tables_read[table] += 1
        key = identity(sql)
        record = occurrences.setdefault(
            key,
            {"table": table, "operation": op, "sql": sql, "reads": []},
        )
        kind = "repeat" if record["reads"] else "first"
        record["reads"].append({"seq": seq, "where": where, "kind": kind})

    repeated = [r for r in occurrences.values() if len(r["reads"]) > 1]
    return repeated, tables_read


def count_repeats(repeated):
    return sum(len(record["reads"]) - 1 for record in repeated)


def build_report(sessions, show_binds):
    flows = {}
    for request_id, session in sessions.items():
        repeated, tables_read = analyse_session(session["calls"])
        repeats = count_repeats(repeated)
        flow = flows.setdefault(
            session["flow"],
            {
                "sessions": 0,
                "reads": 0,
                "duplicates": 0,
                "worst_per_session": 0,
                "tables": collections.Counter(),
                "worst_request": None,
            },
        )
        flow["sessions"] += 1
        flow["reads"] += sum(tables_read.values())
        flow["duplicates"] += repeats
        for record in repeated:
            flow["tables"][record["table"]] += len(record["reads"]) - 1
        if repeats > flow["worst_per_session"]:
            flow["worst_per_session"] = repeats
            flow["worst_request"] = {
                "request_id": request_id,
                "duplicates": [
                    {
                        "table": record["table"],
                        "operation": record["operation"],
                        "sql": redact(record["sql"], show_binds),
                        "repeats": len(record["reads"]),
                        "reads": [
                            {"kind": read["kind"], "where": read["where"]}
                            for read in record["reads"]
                        ],
                    }
                    for record in sorted(repeated, key=lambda r: -len(r["reads"]))
                ],
            }

    return {
        "flows": {
            name: {
                "sessions": data["sessions"],
                "reads": data["reads"],
                "duplicates": data["duplicates"],
                "worst_per_session": data["worst_per_session"],
                "tables": dict(data["tables"].most_common()),
                "worst_request": data["worst_request"],
            }
            for name, data in flows.items()
        }
    }


def build_reported_report(reported):
    """Roll the router's own reports up per flow.

    Each session reports once, so a row is a session that read something twice;
    sessions that read nothing twice never log and are not counted. The budget
    pins the worst single session, which that does not affect.
    """
    flows = {}
    for session in reported:
        duplicates = session["duplicates"]
        repeats = sum(duplicate["repeats"] for duplicate in duplicates)
        flow = flows.setdefault(
            session["flow"],
            {
                "sessions": 0,
                "reads": 0,
                "duplicates": 0,
                "worst_per_session": 0,
                "tables": collections.Counter(),
                "worst_request": None,
            },
        )
        flow["sessions"] += 1
        flow["duplicates"] += repeats
        for duplicate in duplicates:
            flow["tables"][duplicate["table"]] += duplicate["repeats"]
        if repeats > flow["worst_per_session"]:
            flow["worst_per_session"] = repeats
            flow["worst_request"] = {
                "request_id": session["request_id"],
                "duplicates": [
                    {
                        "table": duplicate["table"],
                        "operation": "select",
                        "sql": "",
                        "repeats": duplicate["repeats"] + 1,
                        "reads": [{"kind": "first", "where": duplicate["first_read_from"]}]
                        + [
                            {"kind": "repeat", "where": origin}
                            for origin in duplicate["re_read_from"]
                        ],
                    }
                    for duplicate in sorted(
                        duplicates, key=lambda entry: -entry["repeats"]
                    )
                ],
            }

    return {
        "flows": {
            name: {
                "sessions": data["sessions"],
                "reads": data["reads"],
                "duplicates": data["duplicates"],
                "worst_per_session": data["worst_per_session"],
                "tables": dict(data["tables"].most_common()),
                "worst_request": data["worst_request"],
            }
            for name, data in flows.items()
        }
    }


def summarise_paths(reads):
    """Reduce a group's call paths to what differs between them.

    Two reads of one record share most of their chain -- same root, same flow,
    same storage method at the leaf -- and differ in the middle, which is the
    part worth reading. Returns the shared head, the middle per read, and the
    shared tail.
    """
    chains = [read["where"].split(">") for read in reads]
    if len(chains) < 2:
        return None, [">".join(chain) for chain in chains], None

    head = 0
    while all(len(chain) > head + 1 for chain in chains) and (
        len({chain[head] for chain in chains}) == 1
    ):
        head += 1
    tail = 0
    while all(len(chain) > head + tail + 1 for chain in chains) and (
        len({chain[-1 - tail] for chain in chains}) == 1
    ):
        tail += 1

    # The guards above always leave at least one element per chain, so every
    # middle is non-empty -- two identical paths keep their last element rather
    # than collapsing to nothing.
    middles = [">".join(chain[head : len(chain) - tail]) for chain in chains]
    return (
        ">".join(chains[0][:head]) or None,
        middles,
        ">".join(chains[0][len(chains[0]) - tail :]) if tail else None,
    )


def print_report(report, sessions, query_lines, uncorrelated, reported):
    flows = report["flows"]
    if reported:
        print(
            f"{reported} duplicate reads reported by the router"
            f" across {len(report['flows'])} flows.\n"
        )
    else:
        print(f"Parsed {query_lines} query lines across {len(sessions)} API sessions.\n")
    if uncorrelated:
        print(
            f"{uncorrelated} statements carried no request_id and were skipped. Background\n"
            "work (drainer, scheduler, consumer) has no API session and is expected here;\n"
            "if this is the whole file, the root span is not stamping request_id.\n"
        )
    if not flows:
        print("No duplicate reads found, and no statement lines to infer any from.")
        print("Nothing to report is the good outcome, but check the log is the right")
        print("one first: records must be JSON, which the file sink always is:")
        print("  ROUTER__LOG__FILE__ENABLED=true   # writes logs/debug.log.<hour>")
        print("WARN level is enough for what the router reports itself; DEBUG is")
        print("only needed for the inferred fallback.")
        return

    ranked = sorted(
        flows.items(),
        key=lambda kv: (-kv[1]["worst_per_session"], -kv[1]["duplicates"]),
    )
    width = max(len(name) for name in flows)
    if reported:
        # Only sessions that hit a duplicate reach a WARN log, and the funnel
        # does not warn about re-reads a write justified, so neither a total
        # read count nor an after-write column means anything here.
        print(f"{'flow':<{width}} {'sessions':>8} {'worst':>7} {'total':>7}")
        for name, data in ranked:
            print(
                f"{name:<{width}} {data['sessions']:>8}"
                f" {data['worst_per_session']:>7} {data['duplicates']:>7}"
            )
    else:
        print(f"{'flow':<{width}} {'sessions':>8} {'reads':>6} {'worst':>7} {'total':>7}")
        for name, data in ranked:
            print(
                f"{name:<{width}} {data['sessions']:>8} {data['reads']:>6}"
                f" {data['worst_per_session']:>7} {data['duplicates']:>7}"
            )

    print(
        "\nworst = most duplicate reads seen in one session; this is what a"
        "\n        baseline pins, since totals scale with suite size."
    )

    shown = 0
    for name, data in ranked:
        worst = data["worst_request"]
        if not worst or not data["worst_per_session"]:
            continue
        print(f"\n{name} -- worst session {worst['request_id']}")
        for duplicate in worst["duplicates"]:
            reads = duplicate["reads"]
            if duplicate.get("sql"):
                print(
                    f"  {duplicate['table']} read {duplicate['repeats']}x:"
                    f" {duplicate['sql']}"
                )
            else:
                recurred = f" (x{duplicate['repeats']})" if duplicate["repeats"] > 1 else ""
                print(f"  {duplicate['table']}{recurred}")
            head, middles, tail = summarise_paths(reads)
            shared = " ... ".join(part for part in (head, tail) if part)
            if shared:
                print(f"      shared      {shared}")
            for read, middle in zip(reads, middles):
                print(f"      {read['kind']:<11} {middle}")
        shown += 1
        if shown == 3:
            break


def check_baseline(report, baseline, strict):
    """Return (regressions, improvements, unknown_flows)."""
    pinned = baseline.get("flows", {})
    regressions, improvements, unknown = [], [], []
    for name, data in sorted(report["flows"].items()):
        worst = data["worst_per_session"]
        if name not in pinned:
            if worst > 0 or strict:
                unknown.append((name, worst))
            continue
        allowed = pinned[name]
        if worst > allowed:
            regressions.append((name, worst, allowed))
        elif worst < allowed:
            improvements.append((name, worst, allowed))
    return regressions, improvements, unknown


PI = (
    'SELECT \\"payment_intent\\".\\"id\\" FROM \\"payment_intent\\" '
    'WHERE (\\"payment_intent\\".\\"payment_id\\" = $1) -- binds: [%s]'
)
PA = (
    'SELECT \\"payment_attempt\\".\\"id\\" FROM \\"payment_attempt\\" '
    'WHERE (\\"payment_attempt\\".\\"attempt_id\\" = $1) -- binds: [att_1]'
)


def _fixture(request_id, flow, query, where=None, fn=None, nested=False):
    fields = [f'"request_id":"{request_id}"', f'"flow":"{flow}"']
    if where is not None and not nested:
        fields.append(f'"span_path":"{where}"')
    if fn is not None:
        fields.append(f'"fn":"{fn}"')
    inner = f'"query":"{query}"'
    if where is not None and nested:
        inner += f',"span_path":"{where}"'
    fields.append(f'"extra":{{{inner}}}')
    return '{"level":"DEBUG",' + ",".join(fields) + "}"


CREATE_GET = "ROOT_SPAN>payments_create>get_trackers>find_payment_intent_by_payment_id"
CREATE_UPDATE = "ROOT_SPAN>payments_create>update_trackers>find_payment_intent_by_payment_id"
CONFIRM_GET = "ROOT_SPAN>payments_confirm>get_trackers>find_payment_attempt"
CONFIRM_RESPONSE = "ROOT_SPAN>payments_confirm>payment_response>find_payment_attempt"
CONFIRM_SERIALISE = "ROOT_SPAN>payments_confirm>generate_response>find_payment_attempt"

SELF_TEST_LOG = [
    # Same intent, same lookup, two call sites -> one duplicate.
    _fixture("r1", "PaymentsCreate", PI % "pay_1", CREATE_GET),
    _fixture("r1", "PaymentsCreate", PI % "pay_1", CREATE_UPDATE),
    # Different bind -> a different record, not a duplicate.
    _fixture("r1", "PaymentsCreate", PI % "pay_2", CREATE_GET),
    # A write in between is no defence: the update returned the row already.
    _fixture("r2", "PaymentsConfirm", PA, CONFIRM_GET),
    _fixture(
        "r2",
        "PaymentsConfirm",
        'UPDATE \\"payment_attempt\\" SET \\"status\\" = $1 '
        'WHERE (\\"payment_attempt\\".\\"attempt_id\\" = $2) -- binds: [charged, att_1]',
        "ROOT_SPAN>payments_confirm>update_trackers>update_payment_attempt",
    ),
    _fixture("r2", "PaymentsConfirm", PA, CONFIRM_RESPONSE),
    # ... and a third read repeats it again.
    _fixture("r2", "PaymentsConfirm", PA, CONFIRM_SERIALISE, nested=True),
    # No span_path (logs predating the formatter change) -> fall back to `fn`.
    _fixture(
        "r3",
        "PaymentsList",
        'SELECT COUNT(*) FROM \\"payment_intent\\" '
        'WHERE (\\"payment_intent\\".\\"merchant_id\\" = $1) -- binds: [m_1]',
        fn="get_filtered_payment_intents_attempt",
    ),
    _fixture(
        "r3",
        "PaymentsList",
        'SELECT COUNT(*) FROM \\"payment_intent\\" '
        'WHERE (\\"payment_intent\\".\\"merchant_id\\" = $1) -- binds: [m_1]',
        fn="get_filtered_payment_intents_attempt",
    ),
    # Noise the parser must survive.
    "not json at all",
    '{"level":"INFO","request_id":"r3","message":"no query field here"}',
    '{"level":"DEBUG","extra":{"query":"SELECT 1 FROM \\"configs\\" -- binds: []"}}',
]


def _session_fixture(request_id, flow, duplicates, nested=True):
    """One session's report line, as the router writes it.

    `duplicates` is JSON inside a log field, so the fixture builds the record as
    data and serialises it rather than hand-escaping quotes.
    """
    fields = {
        "level": "WARN",
        "request_id": request_id,
        "flow": flow,
        "message": "records read more than once in this API session",
    }
    payload = {
        "duplicate_reads": sum(entry["repeats"] for entry in duplicates),
        "records": len(duplicates),
        "duplicates": json.dumps(duplicates),
    }
    if nested:
        fields["extra"] = payload
    else:
        fields.update(payload)
    return json.dumps(fields)


CONFIRM_WORST = [
    {
        "table": "payment_intent",
        "repeats": 2,
        "first_read_from": "ROOT>get_trackers>find_pi",
        "re_read_from": ["ROOT>call_connector>find_pi", "ROOT>generate_response>find_pi"],
    },
    {
        "table": "merchant_key_store",
        "repeats": 1,
        "first_read_from": "ROOT>authenticate>find_mks",
        "re_read_from": ["ROOT>get_trackers>find_mks"],
    },
]

REPORTED_LOG = [
    _session_fixture("r9", "PaymentsConfirm", CONFIRM_WORST),
    # A quieter session of the same flow must not become the budget.
    _session_fixture(
        "r10",
        "PaymentsConfirm",
        [
            {
                "table": "payment_intent",
                "repeats": 1,
                "first_read_from": "ROOT>get_trackers>find_pi",
                "re_read_from": ["ROOT>generate_response>find_pi"],
            }
        ],
    ),
    # Fields at the top level (build without log_custom_entries_to_extra).
    _session_fixture(
        "r11",
        "PaymentsCreate",
        [
            {
                "table": "merchant_account",
                "repeats": 1,
                "first_read_from": "ROOT>a>find_ma",
                "re_read_from": ["ROOT>b>find_ma"],
            }
        ],
        nested=False,
    ),
    # Malformed detail must be skipped, not crash the run.
    '{"level":"WARN","request_id":"r12","flow":"PaymentsCreate","extra":{"duplicates":"not json"}}',
    '{"level":"WARN","request_id":"r13","flow":"PaymentsCreate","extra":{"duplicates":"[42]"}}',
    '{"level":"WARN","flow":"PaymentsCreate","extra":{"duplicates":"[{\\"table\\":\\"t\\",\\"repeats\\":1}]"}}',
]


def self_test():
    sessions, _reported, query_lines, uncorrelated = parse_log(SELF_TEST_LOG)
    report = build_report(sessions, show_binds=False)
    flows = report["flows"]
    failures = []

    checked = []

    def check(label, actual, expected):
        checked.append(label)
        if actual != expected:
            failures.append(f"{label}: expected {expected!r}, got {actual!r}")

    # The line with no request_id is unattributable and must be dropped.
    check("query lines", query_lines, 9)
    check("uncorrelated", uncorrelated, 1)
    check("sessions", len(sessions), 3)

    check("create duplicates", flows["PaymentsCreate"]["duplicates"], 1)
    check("create reads", flows["PaymentsCreate"]["reads"], 3)

    # Three reads of one record: two repeats, the intervening write notwithstanding.
    check("confirm duplicates", flows["PaymentsConfirm"]["duplicates"], 2)

    check("list duplicates", flows["PaymentsList"]["duplicates"], 1)
    check("list table", flows["PaymentsList"]["tables"], {"payment_intent": 1})

    # The whole point of the chain: which call site read it first, which re-read it.
    create = flows["PaymentsCreate"]["worst_request"]["duplicates"]
    check("create duplicate count", len(create), 1)
    check(
        "create call sites",
        [(read["kind"], read["where"]) for read in create[0]["reads"]],
        [("first", CREATE_GET), ("repeat", CREATE_UPDATE)],
    )
    confirm = flows["PaymentsConfirm"]["worst_request"]["duplicates"]
    check(
        "confirm call sites",
        [(read["kind"], read["where"]) for read in confirm[0]["reads"]],
        [
            ("first", CONFIRM_GET),
            ("repeat", CONFIRM_RESPONSE),
            ("repeat", CONFIRM_SERIALISE),
        ],
    )
    check(
        "fn fallback",
        flows["PaymentsList"]["worst_request"]["duplicates"][0]["reads"][1]["where"],
        "get_filtered_payment_intents_attempt",
    )
    wide = ", ".join(f'"t"."c{index}"' for index in range(8))
    check(
        "wide projection collapsed",
        "<8 columns>" in redact(f'SELECT {wide} FROM "t" WHERE 1', show_binds=False),
        True,
    )
    check(
        "narrow projection kept",
        redact('SELECT "t"."a", "t"."b" FROM "t" WHERE 1', show_binds=False),
        'SELECT "t"."a", "t"."b" FROM "t" WHERE 1',
    )
    check("count kept", redact('SELECT COUNT(*) FROM "t"', show_binds=False), 'SELECT COUNT(*) FROM "t"')

    check("count op", classify_sql('SELECT COUNT(*) FROM "x"')[0], "count")
    check("delete table", classify_sql('DELETE FROM "x" WHERE 1')[1], "x")
    check("insert table", classify_sql('INSERT INTO "x" ("a") VALUES ($1)')[1], "x")
    check("update table", classify_sql('UPDATE "x" SET "a" = $1')[1], "x")
    check(
        "binds redacted",
        "pay_1" in redact('SELECT 1 FROM "t" -- binds: [pay_1]', show_binds=False),
        False,
    )
    check(
        "binds kept",
        "pay_1" in redact('SELECT 1 FROM "t" -- binds: [pay_1]', show_binds=True),
        True,
    )

    baseline = {"flows": {"PaymentsCreate": 0, "PaymentsConfirm": 5}}
    regressions, improvements, unknown = check_baseline(report, baseline, strict=False)
    check("regressions", [r[0] for r in regressions], ["PaymentsCreate"])
    check("improvements", [i[0] for i in improvements], ["PaymentsConfirm"])
    check("unknown", [u[0] for u in unknown], ["PaymentsList"])

    check(
        "shared head and tail stripped",
        summarise_paths(
            [
                {"where": "ROOT>confirm>get_trackers@a.rs:1>find_pi@b.rs:2"},
                {"where": "ROOT>confirm>respond@a.rs:9>find_pi@b.rs:2"},
            ]
        ),
        ("ROOT>confirm", ["get_trackers@a.rs:1", "respond@a.rs:9"], "find_pi@b.rs:2"),
    )
    check(
        "identical paths keep a middle",
        summarise_paths([{"where": "ROOT>a>b"}, {"where": "ROOT>a>b"}]),
        ("ROOT>a", ["b", "b"], None),
    )
    check(
        "nothing shared stays whole",
        summarise_paths([{"where": "a"}, {"where": "b"}]),
        (None, ["a", "b"], None),
    )

    _sessions, reported, _lines, _orphans = parse_log(REPORTED_LOG)
    # Three well-formed sessions; the malformed and uncorrelated lines are skipped.
    check("reported sessions parsed", len(reported), 3)
    reported_report = build_reported_report(reported)["flows"]
    check("reported flows", sorted(reported_report), ["PaymentsConfirm", "PaymentsCreate"])
    confirm = reported_report["PaymentsConfirm"]
    check("reported sessions", confirm["sessions"], 2)
    check("reported total", confirm["duplicates"], 4)
    # The budget is the worst single session, not the total across the suite.
    check("reported worst", confirm["worst_per_session"], 3)
    check("reported worst session", confirm["worst_request"]["request_id"], "r9")
    check(
        "reported call sites",
        [
            (read["kind"], read["where"])
            for read in confirm["worst_request"]["duplicates"][0]["reads"]
        ],
        [
            ("first", "ROOT>get_trackers>find_pi"),
            ("repeat", "ROOT>call_connector>find_pi"),
            ("repeat", "ROOT>generate_response>find_pi"),
        ],
    )
    check(
        "repeats weigh the table tally",
        confirm["tables"],
        {"payment_intent": 3, "merchant_key_store": 1},
    )
    check(
        "reported top-level fields",
        reported_report["PaymentsCreate"]["tables"],
        {"merchant_account": 1},
    )

    if failures:
        print("self-test FAILED")
        for failure in failures:
            print("  " + failure)
        return 1
    print(f"self-test passed ({len(checked)} checks)")
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(
        description=(
            "Roll up the duplicate record reads the router reports, per flow, "
            "for a budget or a sweep. The router does the detecting."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("logs", nargs="*", help="router log files (JSON lines)")
    parser.add_argument("--json", metavar="FILE", help="write the report as JSON")
    parser.add_argument("--baseline", metavar="FILE", help="per-flow budget to enforce")
    parser.add_argument(
        "--update-baseline",
        action="store_true",
        help="rewrite --baseline from this run instead of enforcing it",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="fail on flows absent from the baseline, even with zero duplicates",
    )
    parser.add_argument(
        "--show-binds",
        action="store_true",
        help="print bind values (they may contain merchant data)",
    )
    parser.add_argument("--self-test", action="store_true", help="verify the analyser")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()
    if not args.logs:
        parser.error("no log files given (or pass --self-test)")

    lines = []
    for path in args.logs:
        try:
            with open(path, "r", errors="replace") as handle:
                lines.extend(handle.readlines())
        except OSError as error:
            print(f"cannot read {path}: {error}", file=sys.stderr)
            return 2

    sessions, reported, query_lines, uncorrelated = parse_log(lines)
    # What the router reported wins: that is what the running service acted on.
    # Inference is for logs from a build without the ledger.
    if reported:
        report = build_reported_report(reported)
        reported_total = sum(
            duplicate["repeats"]
            for session in reported
            for duplicate in session["duplicates"]
        )
    else:
        report = build_report(sessions, args.show_binds)
        reported_total = 0
    print_report(report, sessions, query_lines, uncorrelated, reported_total)

    if args.json:
        report["totals"] = {
            "source": "reported" if reported else "inferred",
            "reported_sessions": len(reported),
            "sessions": len(sessions),
            "query_lines": query_lines,
            "uncorrelated_statements": uncorrelated,
        }
        with open(args.json, "w") as handle:
            json.dump(report, handle, indent=2, sort_keys=True)
            handle.write("\n")
        print(f"\nwrote {args.json}")

    if not args.baseline:
        return 0

    if args.update_baseline:
        pinned = {
            name: data["worst_per_session"]
            for name, data in sorted(report["flows"].items())
        }
        with open(args.baseline, "w") as handle:
            json.dump(
                {
                    "_comment": (
                        "Max duplicate reads allowed in a single API session, per "
                        "flow. Regenerate with "
                        "scripts/duplicate_reads.py <log> --baseline <this file> "
                        "--update-baseline. Lower a number when you remove a duplicate "
                        "read; never raise one without saying why."
                    ),
                    "flows": pinned,
                },
                handle,
                indent=2,
                sort_keys=True,
            )
            handle.write("\n")
        print(f"\nwrote baseline {args.baseline} ({len(pinned)} flows)")
        return 0

    try:
        with open(args.baseline) as handle:
            baseline = json.load(handle)
    except OSError as error:
        print(f"cannot read baseline {args.baseline}: {error}", file=sys.stderr)
        return 2

    regressions, improvements, unknown = check_baseline(report, baseline, args.strict)

    for name, worst, allowed in improvements:
        print(
            f"\nimproved: {name} now at {worst} duplicate reads"
            f" (baseline {allowed}) -- lower the baseline"
        )
    for name, worst in unknown:
        print(f"\nunpinned flow: {name} at {worst} duplicate reads")
    for name, worst, allowed in regressions:
        print(
            f"\nREGRESSION: {name} reads {worst} records twice in one session"
            f" (baseline {allowed})"
        )

    if regressions or (args.strict and unknown):
        print("\nduplicate-read budget exceeded")
        return 1
    print("\nduplicate-read budget met")
    return 0


if __name__ == "__main__":
    sys.exit(main())
