/**
 * Exact rational arithmetic on bigint. Used so that scores and rates are
 * compared without floating point error. Denominators are always > 0.
 */
export interface Ratio {
  readonly num: bigint;
  readonly den: bigint;
}

export function ratio(num: bigint, den: bigint): Ratio {
  if (den <= 0n) throw new RangeError('ratio denominator must be > 0');
  return { num, den };
}

export function compareRatio(a: Ratio, b: Ratio): -1 | 0 | 1 {
  const l = a.num * b.den;
  const r = b.num * a.den;
  return l < r ? -1 : l > r ? 1 : 0;
}

/** Integer division of n >= 0 by d > 0, rounding half up. */
export function roundHalfUpDiv(n: bigint, d: bigint): bigint {
  if (n < 0n || d <= 0n) throw new RangeError('roundHalfUpDiv needs n >= 0 and d > 0');
  return (2n * n + d) / (2n * d);
}

/**
 * Exact value of a finite, non-negative decimal written in plain notation
 * (no exponent). Config numbers go through this so 0.1 means exactly 1/10.
 */
export function decimalToRatio(value: number): Ratio {
  const text = String(value);
  if (!/^\d+(\.\d+)?$/.test(text)) {
    throw new RangeError(`number ${text} must be a plain non-negative decimal (no exponent)`);
  }
  const [whole = '0', frac = ''] = text.split('.');
  return ratio(BigInt(whole + frac), 10n ** BigInt(frac.length));
}

/** Display only: decimal string with `places` digits, half-up on magnitude. */
export function formatRatio(r: Ratio, places: number): string {
  const negative = r.num < 0n;
  const abs = negative ? -r.num : r.num;
  const scale = 10n ** BigInt(places);
  const scaled = roundHalfUpDiv(abs * scale, r.den);
  const whole = scaled / scale;
  const frac = (scaled % scale).toString().padStart(places, '0');
  const body = places > 0 ? `${whole}.${frac}` : `${whole}`;
  return negative && scaled !== 0n ? `-${body}` : body;
}

/** Display only (probabilities may be floats; money must not use this). */
export function ratioToNumber(r: Ratio): number {
  return Number((r.num * 10n ** 12n) / r.den) / 1e12;
}
