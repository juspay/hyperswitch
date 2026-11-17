import type { ClassCounts } from './outcome';
import { ratio, type Ratio } from './ratio';

/** n = successes + technical failures. Issuer declines and unresolved are excluded. */
export function sampleSize(c: ClassCounts): number {
  return c.success + c.technical_failure;
}

/**
 * Common prior: the pooled success rate (sum successes / sum n) over ALL candidate connectors that
 * have data, 1 when none has. It is the same for every candidate. A leave-one-out ("peers only")
 * prior was rejected: it is anti-correlated with a candidate's own evidence, so with thin data the
 * best performer got the weakest prior and could rank below a connector that had only failed.
 * Callers must pass one entry per distinct connector.
 */
export function pooledPrior(all: readonly ClassCounts[]): Ratio {
  let successes = 0;
  let n = 0;
  for (const c of all) {
    const size = sampleSize(c);
    if (size > 0) {
      successes += c.success;
      n += size;
    }
  }
  return n === 0 ? ratio(1n, 1n) : ratio(BigInt(successes), BigInt(n));
}

/**
 * s_hat = (successes + k*prior) / (n + k), exact. With n + k = 0 (no data and
 * k = 0) there is nothing to shrink with, so the prior itself is returned.
 */
export function shrunkSuccessRate(c: ClassCounts, k: number, prior: Ratio): Ratio {
  const n = BigInt(sampleSize(c));
  const kk = BigInt(k);
  if (n + kk === 0n) return prior;
  return ratio(BigInt(c.success) * prior.den + kk * prior.num, (n + kk) * prior.den);
}
