import type { RoutingDecision } from '../engine/types';
import type { HyperswitchResponse } from '../hyperswitch/client';
import type { Clock } from '../stats/attempt-stats.store';
import type { ExecutionRecord } from './execution';

export type RoutingOverride = 'priority' | 'omitted_fallback' | 'omitted_error';

/** Everything remembered per payment id. No card data: only the engine trace and a request hash. */
export interface TraceRecord {
  paymentId: string;
  createdAtMs: number;
  /** sha256 of the canonical request (see fingerprint.ts). Only the hash is stored, never the body. */
  fingerprint: string;
  /** null when the engine/config threw (override omitted). */
  decision: RoutingDecision | null;
  routingOverride: RoutingOverride;
  error?: string;
  /** Set once Hyperswitch answered 2xx; replays return it instead of re-forwarding. */
  response?: HyperswitchResponse;
  /** What Hyperswitch actually did, set with the response; kept separate from the requested `decision`. */
  execution?: ExecutionRecord;
}

export interface DecisionStore {
  /** Live (not expired) record, or undefined. */
  get(paymentId: string): TraceRecord | undefined;
  save(record: TraceRecord): void;
  /** Pin an id while its request is being processed: it is never expired or evicted until `endRequest`. */
  beginRequest(paymentId: string): void;
  endRequest(paymentId: string): void;
}

const MINUTE_MS = 60_000;

/**
 * In-memory, per process, lost on restart. Bounded: entries expire `ttlMinutes` after they were first
 * saved (injected clock) and at most `maxEntries` are kept, oldest first evicted. In-flight ids are exempt
 * from both, so the store can transiently exceed the cap if every entry is in flight.
 */
export class InMemoryDecisionStore implements DecisionStore {
  private readonly entries = new Map<string, { record: TraceRecord; expiresAt: number }>(); // insertion order = age
  private readonly inFlight = new Map<string, number>(); // id -> concurrent request count

  constructor(
    private readonly ttlMinutes: number,
    private readonly maxEntries: number,
    private readonly clock: Clock,
  ) {
    if (!Number.isInteger(ttlMinutes) || ttlMinutes <= 0) throw new RangeError('ttlMinutes must be a positive integer');
    if (!Number.isInteger(maxEntries) || maxEntries <= 0) throw new RangeError('maxEntries must be a positive integer');
  }

  get(paymentId: string): TraceRecord | undefined {
    this.expire();
    return this.entries.get(paymentId)?.record;
  }

  save(record: TraceRecord): void {
    this.expire();
    const existing = this.entries.get(record.paymentId);
    // Updating keeps the original age and expiry (Map.set on an existing key keeps its position).
    this.entries.set(record.paymentId, { record, expiresAt: existing?.expiresAt ?? this.clock() + this.ttlMinutes * MINUTE_MS });
    for (const id of this.entries.keys()) {
      if (this.entries.size <= this.maxEntries) break;
      if (!this.inFlight.has(id)) this.entries.delete(id);
    }
  }

  beginRequest(paymentId: string): void {
    this.inFlight.set(paymentId, (this.inFlight.get(paymentId) ?? 0) + 1);
  }

  endRequest(paymentId: string): void {
    const n = (this.inFlight.get(paymentId) ?? 0) - 1;
    if (n > 0) this.inFlight.set(paymentId, n);
    else this.inFlight.delete(paymentId);
  }

  /** Plain-JSON copy in age order, for the optional state file. In-flight pins are not saved. */
  exportState(): { record: TraceRecord; expiresAt: number }[] {
    return [...this.entries.values()].map((e) => ({ record: e.record, expiresAt: e.expiresAt }));
  }

  /** Replaces the stored decisions with a previously exported copy (expired ones drop out on next use). */
  importState(rows: readonly { record: TraceRecord; expiresAt: number }[]): void {
    this.entries.clear();
    for (const r of rows) this.entries.set(r.record.paymentId, { record: r.record, expiresAt: r.expiresAt });
  }

  private expire(): void {
    const now = this.clock();
    for (const [id, e] of this.entries) {
      if (this.inFlight.has(id)) continue;
      if (e.expiresAt > now) break; // non-in-flight entries are ordered by expiry
      this.entries.delete(id);
    }
  }
}
