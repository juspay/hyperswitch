import type { Ratio } from './ratio';
import type { ClassCounts } from './outcome';

export interface Candidate {
  readonly connector: string;
  readonly merchantConnectorId: string;
}

export interface RoutingInput {
  readonly paymentId: string;
  readonly currency: string;
  /** Integer minor units. */
  readonly amountMinor: bigint;
  /** Coarse BIN class label chosen by the caller (never a card number). */
  readonly binClass?: string;
  readonly cardType?: string;
  readonly candidates: readonly Candidate[];
}

/** Observed attempts per connector name for the input currency, from the stats window. */
export type ObservedAttempts = Readonly<Record<string, ClassCounts>>;

export type CandidateStatus = 'eligible' | 'unpriced' | 'below_floor';
export type SelectionReason = 'best_score' | 'ranked_by_score' | 'unpriced_last' | 'excluded';
export type FallbackReason = 'no_usable_pricing' | 'reliability_floor_unmet';

export interface CandidateTrace {
  connector: string;
  merchantConnectorId: string;
  /** Minor units as decimal string, null when unpriced. */
  fee: string | null;
  sHat: { num: string; den: string; value: number };
  observed: ClassCounts;
  status: CandidateStatus;
  statusReason: string;
  /** Expected-cost components in minor units, 4 decimals, display only. */
  successTerm: string | null;
  failureTerm: string | null;
  score: string | null;
  rank: number | null;
  selectionReason: SelectionReason;
}

export interface DecisionTrace {
  paymentId: string;
  currency: string;
  amountMinor: string;
  binClass: string | null;
  cardType: string | null;
  /** Configured failure_penalty_pct (never mutated). */
  failurePenaltyPct: number;
  /** failure_penalty_pct/100 * amount, minor units, before the clamp. */
  lEffRaw: string;
  /** Effective failure cost in minor units after clamping up to the highest priced candidate fee. */
  lEff: string;
  lEffClamped: boolean;
  /** Penalty (percent of amount) at which chosen and runner-up tie; null if undefined. */
  breakEvenPenaltyPct: number | null;
  fallbackReason: FallbackReason | null;
  candidates: CandidateTrace[];
  summary: string;
}

export interface RoutingDecision {
  ordered: Candidate[];
  fallbackToHyperswitch: boolean;
  fallbackReason: FallbackReason | null;
  trace: DecisionTrace;
}

/** Internal per-candidate working state passed through the pipeline. */
export interface Evaluated {
  readonly candidate: Candidate;
  readonly fee: bigint | null;
  readonly sHat: Ratio;
  readonly observed: ClassCounts;
}
