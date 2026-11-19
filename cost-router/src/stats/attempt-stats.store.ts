import { emptyCounts, type ClassCounts, type OutcomeClass } from '../engine/outcome';

export type Clock = () => number; // epoch milliseconds

export interface AttemptStatsStore {
  /** Record one classified attempt for (connector, currency) at the clock's current time. */
  record(connector: string, currency: string, outcome: OutcomeClass): void;
  /** Counts per requested connector inside the rolling window; connectors without data get zeros. */
  counts(currency: string, connectors: readonly string[]): Record<string, ClassCounts>;
}

const MINUTE_MS = 60_000; // bucket granularity (an implementation detail, not a routing tunable)

/** One one-minute bucket as stored in the state file. */
export interface StatsBucketState {
  connector: string;
  currency: string;
  /** Minutes since the epoch. */
  minute: number;
  counts: ClassCounts;
}

/**
 * In-memory rolling window built from one-minute buckets. State is lost on
 * restart and is per-process (no sharing between replicas): MVP limits.
 */
export class InMemoryAttemptStatsStore implements AttemptStatsStore {
  // key -> (minute index -> counts)
  private readonly buckets = new Map<string, Map<number, ClassCounts>>();

  constructor(
    private readonly windowMinutes: number,
    private readonly clock: Clock,
  ) {
    if (!Number.isInteger(windowMinutes) || windowMinutes <= 0) {
      throw new RangeError('windowMinutes must be a positive integer');
    }
  }

  record(connector: string, currency: string, outcome: OutcomeClass): void {
    const key = this.key(connector, currency);
    let series = this.buckets.get(key);
    if (!series) this.buckets.set(key, (series = new Map()));
    const minute = this.minute();
    let bucket = series.get(minute);
    if (!bucket) series.set(minute, (bucket = emptyCounts()));
    bucket[outcome] += 1;
    this.prune(series, minute);
  }

  counts(currency: string, connectors: readonly string[]): Record<string, ClassCounts> {
    const minute = this.minute();
    const out: Record<string, ClassCounts> = {};
    for (const connector of connectors) {
      const total = emptyCounts();
      const series = this.buckets.get(this.key(connector, currency));
      if (series) {
        this.prune(series, minute);
        for (const bucket of series.values()) {
          for (const k of Object.keys(total) as OutcomeClass[]) total[k] += bucket[k];
        }
      }
      out[connector] = total;
    }
    return out;
  }

  /** Plain-JSON copy of every bucket, for the optional state file. */
  exportState(): StatsBucketState[] {
    const out: StatsBucketState[] = [];
    for (const [key, series] of this.buckets) {
      const [connector, currency] = key.split('\u0000') as [string, string];
      for (const [minute, counts] of series) out.push({ connector, currency, minute, counts: { ...counts } });
    }
    return out;
  }

  /** Replaces the current buckets with a previously exported copy. */
  importState(rows: readonly StatsBucketState[]): void {
    this.buckets.clear();
    for (const r of rows) {
      const key = this.key(r.connector, r.currency);
      let series = this.buckets.get(key);
      if (!series) this.buckets.set(key, (series = new Map()));
      series.set(r.minute, { ...emptyCounts(), ...r.counts });
    }
  }

  private minute(): number {
    return Math.floor(this.clock() / MINUTE_MS);
  }

  /** Window = the current minute and the (windowMinutes - 1) before it. */
  private prune(series: Map<number, ClassCounts>, nowMinute: number): void {
    for (const m of series.keys()) if (m <= nowMinute - this.windowMinutes) series.delete(m);
  }

  private key(connector: string, currency: string): string {
    return `${connector}\u0000${currency.toUpperCase()}`;
  }
}
