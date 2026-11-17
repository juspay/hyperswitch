import type { RoutingInput } from './types';
import { roundHalfUpDiv } from './ratio';

export interface FeeMatch {
  readonly currency: string; // upper-case ISO 4217
  readonly binClass?: string;
  readonly cardType?: string;
  readonly minAmountMinor?: bigint; // inclusive
  readonly maxAmountMinor?: bigint; // inclusive
}

/** All values are integer minor units (bps = basis points, 100 bps = 1%). */
export interface FeeRule {
  readonly match: FeeMatch;
  readonly bps: bigint;
  readonly fixed: bigint;
  readonly min: bigint;
  readonly cap: bigint | null;
}

/** Number of optional dimensions the rule constrains. More = more specific. */
export function specificity(m: FeeMatch): number {
  return (
    (m.binClass !== undefined ? 1 : 0) +
    (m.cardType !== undefined ? 1 : 0) +
    (m.minAmountMinor !== undefined || m.maxAmountMinor !== undefined ? 1 : 0)
  );
}

/** fee = clamp(round_half_up(amount * bps / 10000) + fixed, min, cap). BigInt: exact for any amount. */
export function calculateFee(rule: FeeRule, amountMinor: bigint): bigint {
  if (amountMinor < 0n) throw new RangeError('amountMinor must be >= 0');
  let fee = roundHalfUpDiv(amountMinor * rule.bps, 10000n) + rule.fixed;
  if (fee < rule.min) fee = rule.min;
  if (rule.cap !== null && fee > rule.cap) fee = rule.cap;
  return fee;
}

function matches(m: FeeMatch, input: RoutingInput): boolean {
  if (m.currency !== input.currency.toUpperCase()) return false;
  if (m.binClass !== undefined && m.binClass !== input.binClass) return false;
  if (m.cardType !== undefined && m.cardType !== input.cardType) return false;
  if (m.minAmountMinor !== undefined && input.amountMinor < m.minAmountMinor) return false;
  if (m.maxAmountMinor !== undefined && input.amountMinor > m.maxAmountMinor) return false;
  return true;
}

/**
 * `rules` must already be ordered most-specific-first (config validation does
 * this and rejects ambiguous overlaps), so the first match wins.
 * Returns null when no rule applies (connector is "unpriced" for this input).
 */
export function resolveFee(rules: readonly FeeRule[] | undefined, input: RoutingInput): bigint | null {
  const rule = rules?.find((r) => matches(r.match, input));
  return rule ? calculateFee(rule, input.amountMinor) : null;
}
