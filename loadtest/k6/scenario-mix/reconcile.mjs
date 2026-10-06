import { DatabaseSync } from "node:sqlite";
import { createReadStream, createWriteStream } from "node:fs";
import { once } from "node:events";
import path from "node:path";

// Streaming RFC4180 parser: quoted commas/newlines and escaped quotes survive chunk boundaries.
export async function* csvRows(file) {
  let field = "", row = [], quoted = false, closedQuote = false, first = true, skipLF = false;
  for await (const chunk of createReadStream(file,{ encoding: "utf8" })) {
    for (const character of chunk) {
      if (first) { first = false; if (character === "\uFEFF") continue; }
      if (skipLF) { skipLF = false; if (character === "\n") continue; }
      if (quoted) {
        if (character === '"') { quoted = false; closedQuote = true; } else field += character;
      } else if (closedQuote && character === '"') {
        quoted = true; closedQuote = false; field += '"';
      } else if (character === ",") {
        row.push(field); field = ""; closedQuote = false;
      } else if (character === "\n" || character === "\r") {
        row.push(field); yield row; row = []; field = ""; closedQuote = false; skipLF = character === "\r";
      } else if (character === '"' && !field && !closedQuote) {
        quoted = true;
      } else {
        if (closedQuote || character === '"') throw new Error("Invalid CSV quoting");
        field += character;
      }
      if (field.length > 1024 * 1024) throw new Error("CSV field exceeds 1 MiB");
    }
  }
  if (quoted) throw new Error("Unterminated CSV quote");
  if (field || row.length || closedQuote) { row.push(field); yield row; }
}
const escaped = (value) => `"${String(value ?? "").replaceAll('"','""')}"`;

export async function reconcile({ db: file, csv, output, allowIncomplete = false, runId = null }) {
  if (path.resolve(csv) === path.resolve(output) || path.resolve(file) === path.resolve(output)) throw new Error("Reconciliation output must differ from its inputs");
  const db = new DatabaseSync(file,{ readOnly: true });
  let out;
  try {
    const runs = db.prepare("SELECT * FROM runs").all();
    const run = runId ? runs.find((r) => r.run_id === runId) : runs.length === 1 ? runs[0] : null;
    if (!run) throw new Error("Specify --run-id when the database contains multiple runs, or check the supplied run ID");
    if (!allowIncomplete && (run.recording_state !== "complete" || run.enqueued !== run.persisted)) throw new Error("Recording is incomplete; use --allow-incomplete to inspect partial data");
    db.exec("PRAGMA temp_store=FILE; CREATE TEMP TABLE csv_rows(row_number INTEGER PRIMARY KEY,payment_id TEXT NOT NULL,status TEXT NOT NULL)");
    const insert = db.prepare("INSERT INTO csv_rows VALUES(?,?,?)");
    let header = false, number = 0, count = 0;
    db.exec("BEGIN");
    for await (const row of csvRows(csv)) {
      number++;
      if (row.length === 1 && row[0] === "") continue;
      if (!header) {
        if (row.length !== 2 || row[0] !== "payment_id" || row[1] !== "status") throw new Error("CSV header must be payment_id,status");
        header = true; continue;
      }
      if (row.length !== 2 || !row[0] || !row[1]) throw new Error(`Invalid CSV row ${number}: payment_id and status are required`);
      insert.run(number,row[0],row[1]); count++;
      if (count % 10000 === 0) { db.exec("COMMIT; BEGIN"); }
    }
    db.exec("COMMIT");
    if (!header || !count) throw new Error("CSV must contain a header and at least one payment");
    db.exec("CREATE INDEX temp.csv_payment_id ON csv_rows(payment_id); CREATE TEMP TABLE csv_groups AS SELECT payment_id,COUNT(*) AS n,COUNT(DISTINCT status) AS statuses FROM csv_rows GROUP BY payment_id; CREATE UNIQUE INDEX temp.csv_group_id ON csv_groups(payment_id)");
    const query = db.prepare(`SELECT c.row_number,c.payment_id,c.status AS csv_status,p.status AS recorded_status,
      CASE WHEN g.n>1 AND g.statuses>1 THEN 'conflicting_duplicate' WHEN g.n>1 THEN 'duplicate'
      WHEN p.payment_id IS NULL THEN 'missing' WHEN p.status IS NOT c.status THEN 'mismatch' ELSE 'match' END AS result
      FROM csv_rows c JOIN csv_groups g USING(payment_id)
      LEFT JOIN payments p ON p.run_id=? AND p.payment_id=c.payment_id ORDER BY c.row_number`);
    const summary = { match: 0, mismatch: 0, missing: 0, duplicate: 0, conflicting_duplicate: 0 };
    out = createWriteStream(output,{ flags: "wx", mode: 0o600 });
    // Attach an error listener immediately; a later drain/finish wait also observes errors.
    let outputError;
    out.on("error",(error) => { outputError = error; });
    async function write(text) {
      if (outputError) throw outputError;
      if (!out.write(text)) await once(out,"drain");
    }
    await write("row_number,payment_id,csv_status,recorded_status,result\n");
    for (const row of query.iterate(run.run_id)) {
      summary[row.result]++;
      await write([row.row_number,row.payment_id,row.csv_status,row.recorded_status,row.result].map(escaped).join(",")+"\n");
    }
    const finished = once(out,"finish"); out.end(); await finished;
    return { run_id: run.run_id, recording_state: run.recording_state, summary,
      exitCode: Object.entries(summary).some(([key,n]) => key !== "match" && n > 0) ? 2 : 0 };
  } finally { if (out && !out.writableFinished) out.destroy(); db.close(); }
}
