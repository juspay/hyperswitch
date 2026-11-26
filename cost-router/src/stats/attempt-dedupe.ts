import type { Clock } from './attempt-stats.store';

const MINUTE_MS = 60_000;

/**
 * Remembers which (paymentId, attemptId) keys were already counted, so re-reading the same
 * payment never double counts. `claim` is synchronous: call it before any await and two
 * interleaved ingestions of the same attempt cannot both succeed.
 *
 * Memory is bounded two ways: entries expire after `ttlMinutes` (injected clock), and the map
 * never holds more than `maxEntries` (oldest inserted evicted first). Both are implementation
 * limits; an evicted or expired attempt that is seen again would be counted again.
 */
export class AttemptDedupe {
  private readonly seen = new Map<string, number>(); // key -> expiry ms; Map keeps insertion order

  constructor(
    private readonly ttlMinutes: number,
    private readonly maxEntries: number,
    private readonly clock: Clock,
  ) {
    if (!Number.isInteger(ttlMinutes) || ttlMinutes <= 0) throw new RangeError('ttlMinutes must be a positive integer');
    if (!Number.isInteger(maxEntries) || maxEntries <= 0) throw new RangeError('maxEntries must be a positive integer');
  }

  /** True the first time a live key is claimed, false for a duplicate. */
  claim(key: string): boolean {
    const now = this.clock();
    this.expire(now);
    if (this.seen.has(key)) return false;
    this.seen.set(key, now + this.ttlMinutes * MINUTE_MS);
    while (this.seen.size > this.maxEntries) {
      this.seen.delete(this.seen.keys().next().value as string);
    }
    return true;
  }

  /** True when a live key was already claimed. Does not claim it. */
  has(key: string): boolean {
    this.expire(this.clock());
    return this.seen.has(key);
  }

  /** Plain-JSON copy in insertion order, for the optional state file. */
  exportState(): { key: string; expiresAt: number }[] {
    return [...this.seen].map(([key, expiresAt]) => ({ key, expiresAt }));
  }

  /** Replaces the remembered keys with a previously exported copy (expired ones drop out on next use). */
  importState(rows: readonly { key: string; expiresAt: number }[]): void {
    this.seen.clear();
    for (const r of rows) this.seen.set(r.key, r.expiresAt);
  }

  get size(): number {
    return this.seen.size;
  }

  // Expiry times are non-decreasing in insertion order (same ttl, monotone clock), so only the head can be stale.
  private expire(now: number): void {
    for (const [key, expiresAt] of this.seen) {
      if (expiresAt > now) break;
      this.seen.delete(key);
    }
  }
}
