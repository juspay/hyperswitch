import { DatabaseSync } from "node:sqlite";
import { readFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import { responseRecord } from "./record.mjs";

export const sql = (name) => readFileSync(new URL(`./recorder/${name}.sql`, import.meta.url), "utf8");
export class FixtureRecorder {
  constructor(file, metadata = {}) {
    this.db = new DatabaseSync(file);
    this.db.exec(sql("schema"));
    this.runId = randomUUID();
    this.count = 0;
    this.metadata = metadata;
    this.db.prepare("INSERT INTO runs(run_id,started_at,metadata) VALUES(?,?,?)").run(this.runId, Date.now(), JSON.stringify(metadata));
    this.insert = this.db.prepare(sql("event"));
    this.customer = this.db.prepare(sql("customer"));
    this.payment = this.db.prepare(sql("payment"));
  }
  record(e) {
    this.db.exec("BEGIN IMMEDIATE");
    try {
      const id = this.insert.run(this.runId,this.count+1,e.flow_id,e.merchant_id,e.operation,e.role,e.observed_at,e.method,e.url,
        e.customer_id,e.merchant_reference_id,e.payment_id,e.amount,e.status,e.api_status_code,e.request_id,e.latency_ms,
        e.message,e.error_body,e.entity_type).lastInsertRowid;
      if (e.entity_type === "customer" && e.customer_id) this.customer.run(this.runId,e.customer_id,e.merchant_id,e.merchant_reference_id,e.api_status_code,e.request_id,e.message,e.observed_at,id);
      if (e.entity_type === "payment" && e.payment_id) this.payment.run(this.runId,e.payment_id,e.merchant_id,e.customer_id,e.amount,e.status,e.role,e.api_status_code,e.request_id,e.message,e.observed_at,id);
      this.db.prepare("UPDATE runs SET enqueued=?,persisted=? WHERE run_id=?").run(this.count+1,this.count+1,this.runId);
      this.db.exec("COMMIT");
      this.count += 1;
      return id;
    } catch (error) {
      try { this.db.exec("ROLLBACK"); } catch (_) { /* SQLite may already have rolled back */ }
      error.recordingFatal = true; throw error;
    }
  }
  close(error = null) {
    this.db.prepare("UPDATE runs SET recording_state=?,ended_at=?,error=?,metadata=? WHERE run_id=?")
      .run(error ? "incomplete" : "complete",Date.now(),error ? String(error) : null,JSON.stringify(this.metadata),this.runId);
    this.db.close();
  }
}

export async function recordedFetch(recorder, url, options = {}, context = {}) {
  const start = performance.now();
  let response;
  let text = "";
  let transportError;
  try {
    response = await fetch(url, { redirect: "manual", ...options, signal: options.signal || AbortSignal.timeout(30000) });
    text = await response.clone().text();
  } catch (error) { transportError = error; }
  if (recorder) {
    let request = {};
    try { request = JSON.parse(options.body || "{}"); } catch (_) { /* non-JSON request */ }
    recorder.record(responseRecord({ flow_id: context.flow_id || randomUUID(), ...context }, options.method || "GET",url,request,
      response ? { status: response.status, headers: Object.fromEntries(response.headers) } : null,text,
      performance.now() - start,transportError?.message));
  }
  if (transportError) throw transportError;
  return response;
}
