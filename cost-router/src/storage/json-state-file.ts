import { closeSync, existsSync, mkdirSync, openSync, readFileSync, renameSync, unlinkSync, writeFileSync, writeSync } from 'node:fs';
import { dirname } from 'node:path';
import { z } from 'zod';
import { OUTCOME_CLASSES, type ClassCounts } from '../engine/outcome';
import type { TraceRecord } from '../payments/decision.store';
import type { StatsBucketState } from '../stats/attempt-stats.store';

/**
 * Optional JSON-file persistence for the MVP: attempt statistics, attempt de-duplication and stored
 * decisions live in ONE file so they survive a restart together (saving only the counters would let a
 * restart re-count attempts and forget idempotent replays).
 *
 * - Safe writes: the whole file is written to a temp file and then renamed over the old one, so a crash
 *   mid-write leaves the previous file intact.
 * - One process only: a `<file>.lock` holding the owner's pid is created exclusively at startup. A lock whose
 *   pid is no longer running (crash, killed process) is taken over; a live one stops startup.
 * - Speed: the file is rewritten after each decision and each completed payment. Fine for a demo or low
 *   volume, not for high throughput (see docs/KNOWN_LIMITATIONS.md).
 */
export interface PersistedState {
  version: 1;
  savedAtMs: number;
  stats: StatsBucketState[];
  dedupe: { key: string; expiresAt: number }[];
  decisions: { record: TraceRecord; expiresAt: number }[];
}

export class StateFileError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'StateFileError';
  }
}

const counts = z
  .object(Object.fromEntries(OUTCOME_CLASSES.map((c) => [c, z.number().int().nonnegative().optional()])) as Record<string, z.ZodOptional<z.ZodNumber>>)
  .strict();

const stateSchema = z
  .object({
    version: z.literal(1),
    savedAtMs: z.number(),
    stats: z.array(z.object({ connector: z.string().min(1), currency: z.string().min(1), minute: z.number().int(), counts }).strict()),
    dedupe: z.array(z.object({ key: z.string().min(1), expiresAt: z.number() }).strict()),
    decisions: z.array(
      z
        .object({
          record: z
            .object({
              paymentId: z.string().min(1),
              createdAtMs: z.number(),
              fingerprint: z.string().min(1),
              routingOverride: z.enum(['priority', 'omitted_fallback', 'omitted_error']),
            })
            .passthrough(),
          expiresAt: z.number(),
        })
        .strict(),
    ),
  })
  .strict();

function pidAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    return (e as NodeJS.ErrnoException).code === 'EPERM'; // exists but owned by someone else
  }
}

export class JsonStateFile {
  private readonly lockPath: string;
  private locked = false;

  constructor(readonly path: string) {
    this.lockPath = `${path}.lock`;
  }

  /** Takes the single-writer lock. Returns a warning when a stale lock was taken over. Throws if another live process holds it. */
  acquireLock(): string | null {
    mkdirSync(dirname(this.path), { recursive: true });
    let warning: string | null = null;
    for (let attempt = 0; attempt < 2; attempt++) {
      try {
        const fd = openSync(this.lockPath, 'wx');
        writeSync(fd, String(process.pid));
        closeSync(fd);
        this.locked = true;
        process.once('exit', () => this.releaseLock());
        return warning;
      } catch (e) {
        if ((e as NodeJS.ErrnoException).code !== 'EEXIST') throw new StateFileError(`cannot create lock ${this.lockPath}: ${(e as Error).message}`);
        const owner = Number(readFileSync(this.lockPath, 'utf8').trim());
        if (Number.isInteger(owner) && owner > 0 && pidAlive(owner)) {
          throw new StateFileError(
            `state file ${this.path} is in use by process ${owner} (lock ${this.lockPath}); only one cost-router may use a state file`,
          );
        }
        unlinkSync(this.lockPath);
        warning = `took over a stale lock ${this.lockPath} (process ${Number.isNaN(owner) ? 'unknown' : owner} is not running)`;
      }
    }
    throw new StateFileError(`could not acquire lock ${this.lockPath}`);
  }

  releaseLock(): void {
    if (!this.locked) return;
    this.locked = false;
    try {
      if (readFileSync(this.lockPath, 'utf8').trim() === String(process.pid)) unlinkSync(this.lockPath);
    } catch {
      /* already gone */
    }
  }

  /** null when the file does not exist yet (first start). Throws StateFileError on an unreadable or invalid file. */
  load(): PersistedState | null {
    if (!existsSync(this.path)) return null;
    let raw: unknown;
    try {
      raw = JSON.parse(readFileSync(this.path, 'utf8'));
    } catch {
      throw new StateFileError(`state file ${this.path} is not valid JSON; fix it or delete it to start with empty statistics`);
    }
    const parsed = stateSchema.safeParse(raw);
    if (!parsed.success) {
      const where = parsed.error.issues.slice(0, 3).map((i) => `${i.path.join('.') || '(root)'}: ${i.message}`);
      throw new StateFileError(`state file ${this.path} has an unexpected shape (${where.join('; ')}); fix it or delete it`);
    }
    const s = parsed.data;
    return {
      version: 1,
      savedAtMs: s.savedAtMs,
      stats: s.stats.map((r) => ({ ...r, counts: r.counts as Partial<ClassCounts> as ClassCounts })),
      dedupe: s.dedupe,
      decisions: s.decisions as unknown as PersistedState['decisions'],
    };
  }

  /** Atomic replace: write a temp file next to the target, then rename it over the target. */
  save(state: PersistedState): void {
    const tmp = `${this.path}.${process.pid}.tmp`;
    writeFileSync(tmp, JSON.stringify(state, null, 2));
    renameSync(tmp, this.path);
  }
}
