import { z } from 'zod';
import { decimalToRatio, type Ratio } from './ratio';
import { specificity, type FeeMatch, type FeeRule } from './fee';

/** Parsed, validated configuration used by the engine. Money is bigint minor units. */
export interface EngineConfig {
  readonly feeTable: ReadonlyMap<string, readonly FeeRule[]>; // rules ordered most-specific-first
  /** failure_penalty_pct as an exact ratio of a percent, e.g. 25.5 -> 255/10. */
  readonly failurePenaltyPct: Ratio;
  readonly minSampleSize: number;
  readonly windowMinutes: number;
  /** Required absolute floor on s_hat, in [0, 1]. 0 is allowed and excludes nobody. */
  readonly minSuccessRate: Ratio;
  readonly issuerDeclineCodes: ReadonlySet<string>;
  /** Implementation limits of the attempt de-duplication memory (not business tunables). */
  readonly attemptDedupeTtlMinutes: number;
  readonly attemptDedupeMaxEntries: number;
  /** Implementation limits of the per-payment decision store (idempotency memory). */
  readonly decisionTtlMinutes: number;
  readonly decisionMaxEntries: number;
}

export class ConfigError extends Error {
  constructor(readonly issues: readonly string[]) {
    super(`invalid cost-router config:\n - ${issues.join('\n - ')}`);
    this.name = 'ConfigError';
  }
}

const minor = z.number().int().safe().nonnegative();
const label = z.string().min(1);

const ruleSchema = z
  .object({
    match: z
      .object({
        currency: z.string().regex(/^[A-Za-z]{3}$/, 'currency must be a 3-letter code'),
        bin_class: label.optional(),
        card_type: label.optional(),
        min_amount_minor: minor.optional(),
        max_amount_minor: minor.optional(),
      })
      .strict(),
    // Above 10000 bps the fee exceeds the payment: almost certainly a percent/bps unit mix-up.
    bps: minor.max(10000),
    fixed: minor,
    min: minor.default(0),
    cap: minor.optional(),
  })
  .strict()
  .superRefine((r, ctx) => {
    if (r.cap !== undefined && r.cap < r.min) {
      ctx.addIssue({ code: 'custom', message: `cap (${r.cap}) must be >= min (${r.min})` });
    }
    const { min_amount_minor: lo, max_amount_minor: hi } = r.match;
    if (lo !== undefined && hi !== undefined && lo > hi) {
      ctx.addIssue({ code: 'custom', message: 'min_amount_minor must be <= max_amount_minor' });
    }
  });

const rawSchema = z
  .object({
    fee_table: z.record(label, z.array(ruleSchema).min(1)),
    // Required, no default: derived from the merchant's own margin/abandonment data.
    // Range 0..100: it is a percent of the payment amount, so >100 means losing more than the
    // payment itself, which is almost certainly a units mistake. 0 is allowed: L_eff is then set
    // by the clamp alone (the highest candidate fee).
    failure_penalty_pct: z
      .number({ required_error: 'failure_penalty_pct is required (no default): net expected cost of a failed first attempt, percent of the payment amount, 0..100' })
      .finite()
      .min(0)
      .max(100),
    min_sample_size: z
      .number({ required_error: 'min_sample_size is required (no default): shrinkage strength k' })
      .int()
      .safe()
      .nonnegative(),
    window_minutes: z
      .number({ required_error: 'window_minutes is required (no default): rolling stats window in minutes' })
      .int()
      .safe()
      .positive(),
    // Required absolute floor on s_hat. 0 is allowed (every connector passes) so "no floor" is an
    // explicit choice rather than a missing value.
    min_success_rate: z
      .number({ required_error: 'min_success_rate is required (no default): absolute floor on the success rate in [0, 1]; 0 disables the floor' })
      .finite()
      .min(0)
      .max(1),
    issuer_decline_codes: z.array(label).default([]),
    // Implementation limits for attempt de-duplication. Defaults: ttl = 2 x window_minutes,
    // cap = 100000 entries (roughly a few MB). ttl must be >= window_minutes.
    attempt_dedupe_ttl_minutes: z.number().int().safe().positive().optional(),
    attempt_dedupe_max_entries: z.number().int().safe().positive().default(100_000),
    // Decision store (idempotent replay + routing-trace). Implementation limits, not business tunables.
    // Defaults: 1440 minutes (24 h) and 100000 entries.
    decision_ttl_minutes: z.number().int().safe().positive().default(1440),
    decision_max_entries: z.number().int().safe().positive().default(100_000),
  })
  .strict();

type RawRule = z.infer<typeof ruleSchema>;

function toRule(r: RawRule): FeeRule {
  const m = r.match;
  const match: FeeMatch = {
    currency: m.currency.toUpperCase(),
    ...(m.bin_class !== undefined && { binClass: m.bin_class }),
    ...(m.card_type !== undefined && { cardType: m.card_type }),
    ...(m.min_amount_minor !== undefined && { minAmountMinor: BigInt(m.min_amount_minor) }),
    ...(m.max_amount_minor !== undefined && { maxAmountMinor: BigInt(m.max_amount_minor) }),
  };
  return {
    match,
    bps: BigInt(r.bps),
    fixed: BigInt(r.fixed),
    min: BigInt(r.min),
    cap: r.cap === undefined ? null : BigInt(r.cap),
  };
}

function compatible<T>(a: T | undefined, b: T | undefined): boolean {
  return a === undefined || b === undefined || a === b;
}

/** True when some input could match both rules. */
function overlaps(a: FeeMatch, b: FeeMatch): boolean {
  if (a.currency !== b.currency) return false;
  if (!compatible(a.binClass, b.binClass) || !compatible(a.cardType, b.cardType)) return false;
  const lows = [a.minAmountMinor, b.minAmountMinor].filter((v): v is bigint => v !== undefined);
  const highs = [a.maxAmountMinor, b.maxAmountMinor].filter((v): v is bigint => v !== undefined);
  const lo = lows.reduce((x, y) => (y > x ? y : x), 0n);
  return highs.length === 0 || lo <= highs.reduce((x, y) => (y < x ? y : x));
}

function describe(m: FeeMatch): string {
  return JSON.stringify(m, (_k, v: unknown) => (typeof v === 'bigint' ? v.toString() : v));
}

/** Throws ConfigError listing every problem. Call once at startup. */
export function parseEngineConfig(raw: unknown): EngineConfig {
  const parsed = rawSchema.safeParse(raw);
  if (!parsed.success) {
    throw new ConfigError(parsed.error.issues.map((i) => `${i.path.join('.') || '(root)'}: ${i.message}`));
  }
  const p = parsed.data;
  const issues: string[] = [];

  const feeTable = new Map<string, readonly FeeRule[]>();
  for (const [connector, rawRules] of Object.entries(p.fee_table)) {
    const rules = rawRules.map(toRule);
    for (let i = 0; i < rules.length; i++) {
      for (let j = i + 1; j < rules.length; j++) {
        const a = rules[i]!.match;
        const b = rules[j]!.match;
        if (specificity(a) === specificity(b) && overlaps(a, b)) {
          issues.push(
            `fee_table.${connector}: rules ${i} and ${j} are duplicate or ambiguous (same specificity, overlapping): ${describe(a)} vs ${describe(b)}`,
          );
        }
      }
    }
    // Array.prototype.sort is stable: most specific first.
    feeTable.set(connector, [...rules].sort((x, y) => specificity(y.match) - specificity(x.match)));
  }

  let failurePenaltyPct: Ratio | undefined;
  let minSuccessRate: Ratio | undefined;
  const ttl = p.attempt_dedupe_ttl_minutes ?? p.window_minutes * 2;
  if (ttl < p.window_minutes) {
    issues.push(`attempt_dedupe_ttl_minutes (${ttl}) must be >= window_minutes (${p.window_minutes})`);
  }
  try {
    failurePenaltyPct = decimalToRatio(p.failure_penalty_pct);
    minSuccessRate = decimalToRatio(p.min_success_rate);
  } catch (e) {
    issues.push((e as Error).message);
  }
  if (issues.length > 0 || failurePenaltyPct === undefined || minSuccessRate === undefined) throw new ConfigError(issues);

  return {
    feeTable,
    failurePenaltyPct,
    minSampleSize: p.min_sample_size,
    windowMinutes: p.window_minutes,
    minSuccessRate,
    issuerDeclineCodes: new Set(p.issuer_decline_codes),
    attemptDedupeTtlMinutes: ttl,
    attemptDedupeMaxEntries: p.attempt_dedupe_max_entries,
    decisionTtlMinutes: p.decision_ttl_minutes,
    decisionMaxEntries: p.decision_max_entries,
  };
}
