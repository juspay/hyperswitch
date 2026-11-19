import type { EngineConfig } from './config';
import type { CandidateStatus, Evaluated, RoutingInput } from './types';
import { compareRatio, ratio, ratioToNumber, roundHalfUpDiv, type Ratio } from './ratio';

export type PricedEvaluated = Evaluated & { readonly fee: bigint };

export interface DecisionContext {
  readonly input: RoutingInput;
  readonly config: EngineConfig;
  /** Effective failure cost in minor units (configured value, clamped up to the highest candidate fee). */
  readonly lEff: bigint;
}

export interface FilterVerdict {
  readonly status: Exclude<CandidateStatus, 'eligible'>;
  readonly reason: string;
}

/** Eligibility stage. Return null to let the candidate pass, a verdict to stop it. First verdict wins. */
export interface CandidateFilter {
  readonly name: string;
  check(candidate: Evaluated, ctx: DecisionContext): FilterVerdict | null;
}

export interface ScoreResult {
  readonly score: Ratio;
  readonly successTerm: Ratio;
  readonly failureTerm: Ratio;
}

/** Scoring stage. Lower score is better. Only called for eligible (priced) candidates. */
export interface Scorer {
  readonly name: string;
  score(candidate: PricedEvaluated, ctx: DecisionContext): ScoreResult;
}

/** Candidates below the absolute success-rate floor (min_success_rate; 0 excludes nobody). */
export const minSuccessRateFilter: CandidateFilter = {
  name: 'min_success_rate',
  check(c, { config }) {
    const floor = config.minSuccessRate;
    if (compareRatio(c.sHat, floor) >= 0) return null;
    return {
      status: 'below_floor',
      reason: `reliability_floor_unmet: s_hat ${ratioToNumber(c.sHat).toFixed(4)} < min_success_rate ${ratioToNumber(floor)}`,
    };
  },
};

/** Candidates with no matching fee rule cannot be costed: they are ranked last, not dropped. */
export const noPricingFilter: CandidateFilter = {
  name: 'no_pricing',
  check(c) {
    return c.fee === null ? { status: 'unpriced', reason: 'no fee rule matches this payment' } : null;
  },
};

/** score = s_hat * fee + (1 - s_hat) * L_eff. Single-connector approximation (no fallback chain). */
export const expectedCostScorer: Scorer = {
  name: 'expected_cost',
  score(c, { lEff }) {
    const { num, den } = c.sHat;
    const successTerm = ratio(num * c.fee, den);
    const failureTerm = ratio((den - num) * lEff, den);
    return { score: ratio(successTerm.num + failureTerm.num, den), successTerm, failureTerm };
  },
};

/** L_eff = failure_penalty_pct / 100 * amount, round half up, in minor units. */
export function configuredFailureCost(config: EngineConfig, amountMinor: bigint): bigint {
  const p = config.failurePenaltyPct;
  return roundHalfUpDiv(amountMinor * p.num, 100n * p.den);
}
