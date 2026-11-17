import type { EngineConfig } from './config';
import { resolveFee } from './fee';
import { emptyCounts } from './outcome';
import {
  configuredFailureCost,
  expectedCostScorer,
  minSuccessRateFilter,
  noPricingFilter,
  type CandidateFilter,
  type DecisionContext,
  type PricedEvaluated,
  type ScoreResult,
  type Scorer,
} from './pipeline';
import { compareRatio, formatRatio, ratio, ratioToNumber, type Ratio } from './ratio';
import { pooledPrior, shrunkSuccessRate } from './reliability';
import type {
  Candidate,
  CandidateStatus,
  CandidateTrace,
  Evaluated,
  FallbackReason,
  ObservedAttempts,
  RoutingDecision,
  RoutingInput,
  SelectionReason,
} from './types';

export interface DecideOptions {
  readonly filters?: readonly CandidateFilter[];
  readonly scorer?: Scorer;
}

const DEFAULT_FILTERS: readonly CandidateFilter[] = [minSuccessRateFilter, noPricingFilter];

function cmpStr(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function cmpId(a: Candidate, b: Candidate): number {
  return cmpStr(a.connector, b.connector) || cmpStr(a.merchantConnectorId, b.merchantConnectorId);
}

interface Row {
  readonly ev: Evaluated;
  status: CandidateStatus;
  reason: string;
  score: ScoreResult | null;
  rank: number | null;
  selection: SelectionReason;
}

const byScore = (a: Row, b: Row): number =>
  compareRatio(a.score!.score, b.score!.score) ||
  compareRatio(b.ev.sHat, a.ev.sHat) || // higher s_hat first
  Number(a.ev.fee! - b.ev.fee!) ||
  cmpId(a.ev.candidate, b.ev.candidate);

const bySHatDesc = (a: Row, b: Row): number =>
  compareRatio(b.ev.sHat, a.ev.sHat) || cmpId(a.ev.candidate, b.ev.candidate);

/**
 * Pure and deterministic: same (input, config, observed) -> same decision.
 * No clock, no randomness, no I/O. Pipeline: evaluate -> filters -> scorer -> rank.
 */
export function decide(
  input: RoutingInput,
  config: EngineConfig,
  observed: ObservedAttempts,
  options: DecideOptions = {},
): RoutingDecision {
  if (input.amountMinor < 0n) throw new RangeError('amountMinor must be >= 0');
  const filters = options.filters ?? DEFAULT_FILTERS;
  const scorer = options.scorer ?? expectedCostScorer;

  // 1. Evaluate: fee and shrunk reliability per candidate (one common prior: pooled rate of all distinct connectors).
  const countsOf = (c: Candidate) => observed[c.connector] ?? emptyCounts();
  const distinctConnectors = [...new Set(input.candidates.map((c) => c.connector))];
  const prior = pooledPrior(distinctConnectors.map((name) => observed[name] ?? emptyCounts()));
  const evaluated: Evaluated[] = input.candidates.map((candidate) => {
    const observedCounts = countsOf(candidate);
    return {
      candidate,
      fee: resolveFee(config.feeTable.get(candidate.connector), input),
      sHat: shrunkSuccessRate(observedCounts, config.minSampleSize, prior),
      observed: { ...observedCounts },
    };
  });

  // 2. Effective failure cost, clamped up to the highest candidate fee (arithmetic on minor units).
  const fees = evaluated.flatMap((e) => (e.fee === null ? [] : [e.fee]));
  const lConfigured = configuredFailureCost(config, input.amountMinor);
  const maxFee = fees.reduce((m, f) => (f > m ? f : m), 0n);
  const lClamped = fees.length > 0 && lConfigured < maxFee;
  const ctx: DecisionContext = { input, config, lEff: lClamped ? maxFee : lConfigured };

  // 3. Filters then scorer.
  const rows: Row[] = evaluated.map((ev) => {
    let verdict = null;
    for (const f of filters) {
      verdict = f.check(ev, ctx);
      if (verdict) break;
    }
    const row: Row = {
      ev,
      status: verdict?.status ?? 'eligible',
      reason: verdict?.reason ?? 'passed all filters',
      score: null,
      rank: null,
      selection: 'excluded',
    };
    // Scores are computed for every priced candidate (also excluded ones) so the trace explains them.
    if (ev.fee !== null) row.score = scorer.score(ev as PricedEvaluated, ctx);
    return row;
  });

  // 4. Rank. No usable pricing, or every priced connector under the reliability floor, means
  // no override: Hyperswitch routes normally (we never rank by success rate alone).
  const eligible = rows.filter((r) => r.status === 'eligible').sort(byScore);
  const unpriced = rows.filter((r) => r.status === 'unpriced').sort(bySHatDesc);
  const anyPriced = fees.length > 0;

  let fallbackReason: FallbackReason | null = null;
  let ordered: Row[] = [];
  if (!anyPriced) fallbackReason = 'no_usable_pricing';
  else if (eligible.length === 0) fallbackReason = 'reliability_floor_unmet';
  else {
    ordered = [...eligible, ...unpriced];
    eligible.forEach((r, i) => (r.selection = i === 0 ? 'best_score' : 'ranked_by_score'));
    unpriced.forEach((r) => (r.selection = 'unpriced_last'));
    ordered.forEach((r, i) => (r.rank = i + 1));
  }

  // 5. Break-even penalty between the top two, when both were scored normally.
  const [first, second] = eligible;
  const breakEvenPenaltyPct = first && second ? breakEven(first, second, input.amountMinor) : null;

  const trace = {
    paymentId: input.paymentId,
    currency: input.currency.toUpperCase(),
    amountMinor: input.amountMinor.toString(),
    binClass: input.binClass ?? null,
    cardType: input.cardType ?? null,
    failurePenaltyPct: ratioToNumber(config.failurePenaltyPct),
    lEffRaw: lConfigured.toString(),
    lEff: ctx.lEff.toString(),
    lEffClamped: lClamped,
    breakEvenPenaltyPct,
    fallbackReason,
    candidates: rows.map(toCandidateTrace),
    summary: summarize(input, ordered, ctx.lEff, lClamped, fallbackReason),
  };

  return {
    ordered: ordered.map((r) => r.ev.candidate),
    fallbackToHyperswitch: fallbackReason !== null,
    fallbackReason,
    trace,
  };
}

/** L* = (s1*f1 - s2*f2) / (s1 - s2), as percent of the amount. Null when s1 == s2 or amount is 0. */
function breakEven(chosen: Row, runnerUp: Row, amount: bigint): number | null {
  const { num: a, den: b } = chosen.ev.sHat;
  const { num: c, den: d } = runnerUp.ev.sHat;
  let num = a * chosen.ev.fee! * d - c * runnerUp.ev.fee! * b;
  let den = a * d - c * b;
  if (den === 0n || amount === 0n) return null;
  if (den < 0n) {
    num = -num;
    den = -den;
  }
  return ratioToNumber(ratio(num * 100n, den * amount));
}

function toCandidateTrace(r: Row): CandidateTrace {
  const money = (v: Ratio | undefined) => (v ? formatRatio(v, 4) : null);
  return {
    connector: r.ev.candidate.connector,
    merchantConnectorId: r.ev.candidate.merchantConnectorId,
    fee: r.ev.fee === null ? null : r.ev.fee.toString(),
    sHat: { num: r.ev.sHat.num.toString(), den: r.ev.sHat.den.toString(), value: ratioToNumber(r.ev.sHat) },
    observed: r.ev.observed,
    status: r.status,
    statusReason: r.reason,
    successTerm: money(r.score?.successTerm),
    failureTerm: money(r.score?.failureTerm),
    score: money(r.score?.score),
    rank: r.rank,
    selectionReason: r.selection,
  };
}

function summarize(
  input: RoutingInput,
  ordered: readonly Row[],
  lEff: bigint,
  clamped: boolean,
  fallbackReason: FallbackReason | null,
): string {
  const head = `payment ${input.paymentId} ${input.currency.toUpperCase()} ${input.amountMinor}`;
  if (fallbackReason === 'no_usable_pricing') {
    return `${head}: no candidate has usable pricing, deferring to Hyperswitch routing`;
  }
  if (fallbackReason === 'reliability_floor_unmet') {
    return `${head}: reliability_floor_unmet, every priced candidate is below min_success_rate, deferring to Hyperswitch routing`;
  }
  const [top, next] = ordered;
  const desc = (r: Row) =>
    `${r.ev.candidate.connector} (score ${r.score ? formatRatio(r.score.score, 2) : 'n/a'}, fee ${r.ev.fee ?? 'n/a'}, s_hat ${ratioToNumber(r.ev.sHat).toFixed(4)})`;
  const tail = `L_eff ${lEff}${clamped ? ' (clamped up to highest fee)' : ''}`;
  return `${head}: chose ${desc(top!)}${next ? ` over ${desc(next)}` : ''}; ${tail}`;
}
