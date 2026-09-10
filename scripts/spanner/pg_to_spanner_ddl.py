#!/usr/bin/env python3
"""Translate a `pg_dump --schema-only` of hyperswitch_db into Spanner PG-dialect DDL.

The 511 diesel migrations cannot be replayed against Spanner: the history is full
of CREATE TYPE, SERIAL and Postgres-only ALTERs. So the source of truth for the
Spanner baseline is the *end state* of a normal local Postgres, dumped and
translated here.

    diesel migration run                          # against local pg
    pg_dump --schema-only --no-owner --no-acl \
        "postgres://db_user:db_pass@localhost:5432/hyperswitch_db" \
        > /tmp/pg_schema.sql
    ./scripts/spanner/pg_to_spanner_ddl.py /tmp/pg_schema.sql > /tmp/spanner.sql

Anything this script cannot translate is emitted as a `-- UNSUPPORTED:` comment
rather than silently dropped, and summarised on stderr. Read that summary: it is
the actual list of decisions a human still has to make.
"""

import re
import sys
from collections import defaultdict

# Postgres enum types become plain varchar. Paired with the router_derive change
# that swaps postgres_type(name=...) for postgres_type(oid=25), the Rust side
# keeps its strong enum types and only the wire representation changes.
ENUM_TARGET = "varchar"

SCALAR_MAP = {
    # Spanner PG has a single 64-bit integer type.
    "smallint": "bigint",
    "integer": "bigint",
    "int": "bigint",
    "int2": "bigint",
    "int4": "bigint",
    "int8": "bigint",
    "bigint": "bigint",
    # Spanner PG has no `timestamp without time zone`.
    "timestamp without time zone": "timestamptz",
    "timestamp with time zone": "timestamptz",
    "timestamp": "timestamptz",
    "timestamptz": "timestamptz",
    # json is not a Spanner PG type; jsonb is.
    "json": "jsonb",
    "jsonb": "jsonb",
    "double precision": "float8",
    "real": "float8",
    "character varying": "varchar",
    "character": "varchar",
    "text": "text",
    "bytea": "bytea",
    "boolean": "bool",
    "numeric": "numeric",
    "date": "date",
}

# Element types Spanner PG accepts inside an array.
# Verified against the Spanner emulator (PGAdapter 0.55.3): jsonb[] round-trips.
ARRAY_OK = {"bool", "bytea", "float8", "bigint", "numeric", "text", "varchar",
            "timestamptz", "date", "jsonb"}

issues = defaultdict(list)

# Populated from the dump's CREATE TYPE statements. A default cast to one of
# these is an enum cast and the cast must be dropped (the column is varchar now);
# a cast to anything else is load-bearing and must be kept, or Spanner reads
# '{}'::jsonb as the string "{}".
enum_names = set()

# Sentinel: the column carries an IDENTITY clause rather than a DEFAULT.
IDENTITY = "__SPANNER_IDENTITY__"

# (table, column) pairs whose default is a sequence, gathered from the
# trailing ALTER TABLE statements pg_dump emits for SERIAL columns.
serial_columns = set()

# Tables whose primary key Postgres does not declare. `blocklist` is the same
# table crates/diesel_models/drop_id.patch fixes up on the Rust side.
PK_OVERRIDE = {"blocklist": "merchant_id, fingerprint_id"}

# diesel's own migration bookkeeping. Meaningless on Spanner, and the leading
# underscores are not a legal Spanner table name anyway.
SKIP_TABLES = {"__diesel_schema_migrations"}


def note(kind, msg):
    issues[kind].append(msg)


def map_type(raw, table, column):
    """Map one Postgres column type to its Spanner PG equivalent."""
    t = raw.strip()

    is_array = t.endswith("[]")
    if is_array:
        t = t[:-2].strip()

    # Strip schema qualification and quoting: public."AttemptStatus" -> AttemptStatus
    bare = re.sub(r'^public\.', '', t)
    quoted_enum = bare.startswith('"') and bare.endswith('"')
    bare_unquoted = bare.strip('"')

    # Carry the length modifier through: character varying(64) -> varchar(64)
    length = None
    m = re.match(r'^(.*?)\s*\((\d+(?:,\s*\d+)?)\)$', bare_unquoted)
    if m:
        bare_unquoted, length = m.group(1).strip(), m.group(2)

    key = bare_unquoted.lower()
    if quoted_enum or (key not in SCALAR_MAP and bare_unquoted[:1].isupper()):
        mapped = ENUM_TARGET          # a user-defined enum type
        length = length or "64"
    elif key in SCALAR_MAP:
        mapped = SCALAR_MAP[key]
        if key in ("smallint", "integer", "int", "int2", "int4"):
            note("int-widened",
                 f"{table}.{column}: {key} -> bigint (Rust i16/i32 needs an i64-compatible shim)")
        if key == "timestamp without time zone":
            note("timestamp", f"{table}.{column}: timestamp -> timestamptz")
        if key == "json":
            note("json", f"{table}.{column}: json -> jsonb")
    else:
        note("unknown-type", f"{table}.{column}: unrecognised type {raw!r}")
        return None

    if length and mapped in ("varchar", "numeric"):
        mapped = f"{mapped}({length})"

    if is_array:
        elem = mapped.split("(")[0]
        if elem not in ARRAY_OK:
            note("array-unsupported",
                 f"{table}.{column}: {elem}[] is not a Spanner PG array type - "
                 f"store the whole list in a single jsonb column instead")
            return None
        mapped = f"{mapped}[]"

    return mapped


def map_default(default, table, column):
    """Map a column DEFAULT, or return None to drop it."""
    d = default.strip()
    if "nextval(" in d:
        # SERIAL has no Spanner equivalent, and a monotonically increasing key
        # pins every write to one split. A bit-reversed identity keeps the
        # auto-generated id the application expects while scattering values.
        note("serial", f"{table}.{column}: SERIAL -> bit-reversed identity")
        return IDENTITY

    # pg_dump writes defaults with an explicit cast:
    #   (now())::timestamp without time zone     '{}'::jsonb
    #   NULL::character varying                  'created'::"AttemptStatus"
    base, cast = d, None
    if "::" in d:
        idx = d.index("::")
        base, cast = d[:idx].strip(), d[idx + 2:].strip()
    if base.startswith("(") and base.endswith(")"):
        base = base[1:-1].strip()

    if re.match(r'^(now\(\)|CURRENT_TIMESTAMP)$', base, re.I):
        return "CURRENT_TIMESTAMP"
    if base.upper() == "NULL":
        return None                     # an explicit NULL default is just the default

    if cast is None:
        return base

    cast_bare = re.sub(r'^public\.', '', cast).strip('"')
    is_array = cast_bare.endswith("[]")
    cast_elem = cast_bare[:-2].strip('"') if is_array else cast_bare

    if cast_elem in enum_names:
        return base                     # enum column is varchar now; drop the cast

    mapped = map_type(cast_bare, table, column)
    if mapped is None:
        note("default", f"{table}.{column}: dropped default {d!r}")
        return None
    # Spanner needs the cast to type the literal: '{}'::jsonb, '{}'::text[]
    return f"{base}::{mapped}"


def main(path):
    sql = open(path).read()

    for m in re.finditer(r'CREATE TYPE (?:public\.)?"?([\w ]+)"?\s+AS ENUM', sql):
        enum_names.add(m.group(1).strip())

    for m in re.finditer(
            r'ALTER TABLE ONLY (?:public\.)?"?(\w+)"?\s+ALTER COLUMN "?(\w+)"?'
            r'\s+SET DEFAULT nextval\(', sql, re.I):
        serial_columns.add((m.group(1), m.group(2)))

    # Table name -> list of rendered column definition lines
    tables = {}
    order = []

    table_re = re.compile(
        r'CREATE TABLE (?:IF NOT EXISTS )?(?:public\.)?"?(\w+)"?\s*\((.*?)\n\);',
        re.S)

    for tm in table_re.finditer(sql):
        table, body = tm.group(1), tm.group(2)
        if table in SKIP_TABLES:
            continue
        order.append(table)
        cols = []
        for line in body.split("\n"):
            line = line.strip().rstrip(",")
            if not line or line.startswith("--"):
                continue
            # Table-level constraints inside CREATE TABLE
            if re.match(r'^(CONSTRAINT|PRIMARY KEY|UNIQUE|CHECK|FOREIGN KEY)\b', line, re.I):
                if re.match(r'^PRIMARY KEY', line, re.I):
                    cols.append(("__pk__", line))
                else:
                    note("constraint", f"{table}: inline constraint deferred - {line[:70]}")
                continue

            cm = re.match(r'^"?(\w+)"?\s+(.+)$', line)
            if not cm:
                note("parse", f"{table}: could not parse column line {line[:70]!r}")
                continue
            column, rest = cm.group(1), cm.group(2)

            not_null = bool(re.search(r'\bNOT NULL\b', rest, re.I))
            rest_no_nn = re.sub(r'\s*\bNOT NULL\b', '', rest, flags=re.I)

            default = None
            dm = re.search(r'\bDEFAULT\s+(.*)$', rest_no_nn, re.I)
            if dm:
                default = map_default(dm.group(1), table, column)
                rest_no_nn = rest_no_nn[:dm.start()]

            mapped = map_type(rest_no_nn, table, column)
            if mapped is None:
                note("column-dropped",
                     f"{table}.{column}: dropped from the baseline - {rest.strip()[:60]}")
                continue

            if (table, column) in serial_columns:
                default = IDENTITY
                note("serial", f"{table}.{column}: SERIAL -> bit-reversed identity")

            piece = f'{column} {mapped}'
            if default is IDENTITY:
                piece += ' GENERATED BY DEFAULT AS IDENTITY (BIT_REVERSED_POSITIVE)'
            elif default is not None:
                piece += f' DEFAULT {default}'
            # An identity column supplies its own value; NOT NULL on top is
            # redundant and Spanner rejects the combination.
            if not_null and default is not IDENTITY:
                piece += ' NOT NULL'
            cols.append((column, piece))
        tables[table] = cols

    # Spanner requires the primary key in the CREATE TABLE body, but pg_dump
    # emits it as a separate ALTER TABLE. Collect those and fold them back in.
    pks = {}
    for m in re.finditer(
            r'ALTER TABLE ONLY (?:public\.)?"?(\w+)"?\s+ADD CONSTRAINT\s+\S+\s+PRIMARY KEY\s*\(([^)]+)\);',
            sql, re.I):
        pks[m.group(1)] = m.group(2).strip()

    unique_indexes = []
    for m in re.finditer(
            r'ALTER TABLE ONLY (?:public\.)?"?(\w+)"?\s+ADD CONSTRAINT\s+"?(\w+)"?\s+UNIQUE\s*\(([^)]+)\);',
            sql, re.I):
        table, cname, cols_u = m.group(1), m.group(2), m.group(3).strip()
        unique_indexes.append(f'CREATE UNIQUE INDEX {cname} ON {table} ({cols_u});')
        note("unique", f"{table}({cols_u}): UNIQUE constraint -> CREATE UNIQUE INDEX {cname}")

    for m in re.finditer(
            r'ALTER TABLE ONLY (?:public\.)?"?(\w+)"?\s+ADD CONSTRAINT\s+\S+\s+FOREIGN KEY',
            sql, re.I):
        note("fk", f"{m.group(1)}: FOREIGN KEY - Spanner supports these but check the parent's key order")

    out = []
    # Header comments are safe: they precede the batch rather than interrupt it.
    out.append("-- Generated by scripts/spanner/pg_to_spanner_ddl.py")
    out.append("-- Spanner PostgreSQL dialect. Read the stderr report for what was skipped.\n")

    for table in order:
        cols = tables[table]
        rendered = []
        inline_pk = None
        for name, piece in cols:
            if name == "__pk__":
                inline_pk = piece
                continue
            rendered.append("    " + piece)

        pk_cols = pks.get(table) or PK_OVERRIDE.get(table)
        pk = inline_pk or (f'PRIMARY KEY ({pk_cols})' if pk_cols else None)
        if pk is None:
            note("no-pk", f"{table}: no primary key found - Spanner requires one. "
                          f"Add it to PK_OVERRIDE at the top of this script.")
            continue

        out.append(f'CREATE TABLE {table} (')
        out.append(",\n".join(rendered) + ",")
        out.append(f'    {pk}')
        out.append(');\n')

    out.extend(unique_indexes)

    # Secondary indexes. The column list has to be parsed by balancing
    # parentheses, not by regex: hyperswitch has expression indexes like
    # `(((event_id)::text = (initial_attempt_id)::text))` whose inner parens
    # defeat a naive `\(([^;]+)\)` match and produce truncated DDL.
    idx_re = re.compile(
        r'CREATE (UNIQUE )?INDEX\s+"?(\w+)"?\s+ON\s+(?:public\.)?"?(\w+)"?'
        r'(?:\s+USING\s+(\w+))?\s*\(', re.I)

    for m in idx_re.finditer(sql):
        uniq, name, table, method = m.group(1), m.group(2), m.group(3), m.group(4)
        if table in SKIP_TABLES:
            continue

        depth, i = 1, m.end()
        while i < len(sql) and depth:
            if sql[i] == '(':
                depth += 1
            elif sql[i] == ')':
                depth -= 1
            i += 1
        cols_raw = sql[m.end():i - 1]
        tail = sql[i:sql.index(';', i)] if ';' in sql[i:] else ''

        if method and method.lower() != 'btree':
            note("index-method", f"{name} on {table}: USING {method} is not supported")
            continue

        cols_clean = re.sub(r'\s+(text_pattern_ops|varchar_pattern_ops)', '', cols_raw).strip()

        # A function call or a cast anywhere in the column list makes this an
        # expression index, which Spanner does not support at all.
        if re.search(r'\w\s*\(', cols_clean) or '::' in cols_clean:
            note("index-expression",
                 f"{name} on {table}: expression index, unsupported - "
                 f"add a generated column or drop it")
            continue

        suffix = ""
        wm = re.search(r'\bWHERE\b\s*(.+)$', tail, re.I | re.S)
        if wm:
            where = wm.group(1).strip().rstrip(')').lstrip('(').strip()
            if re.fullmatch(r'[\w".]+\s+IS NOT NULL'
                            r'(\s+AND\s+[\w".]+\s+IS NOT NULL)*', where, re.I):
                suffix = f" WHERE {where}"
            else:
                note("index-partial",
                     f"{name} on {table}: WHERE {where[:45]} unsupported - "
                     f"emitted as a full index")
        out.append(f'CREATE {uniq or ""}INDEX {name} ON {table} ({cols_clean}){suffix};')

    print("\n".join(out))

    print("\n" + "=" * 72, file=sys.stderr)
    print("TRANSLATION REPORT", file=sys.stderr)
    print("=" * 72, file=sys.stderr)
    for kind in sorted(issues):
        items = issues[kind]
        print(f"\n[{kind}] {len(items)}", file=sys.stderr)
        for i in items[:12]:
            print(f"  - {i}", file=sys.stderr)
        if len(items) > 12:
            print(f"  ... and {len(items) - 12} more", file=sys.stderr)
    print(file=sys.stderr)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: pg_to_spanner_ddl.py <pg_dump-schema-only.sql>")
    main(sys.argv[1])
