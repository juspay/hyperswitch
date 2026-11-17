import { compareRatio, decide, ratio, type RoutingDecision } from '../src/engine';
import { cand, cfg, counts, input } from './helpers';

// Example from the brief: A fee 5000 s=0.96, B fee 2000 s=0.85, k=0 so observed rates are used as-is.
// Expected cost: A = 4800 + 0.04 L, B = 1700 + 0.15 L, break-even L ~ 28181.8.
const AB = {
  A: counts({ success: 96, technical_failure: 4 }),
  B: counts({ success: 85, technical_failure: 15 }),
};
const names = (d: RoutingDecision) => d.ordered.map((c) => c.connector);
const row = (d: RoutingDecision, connector: string) => d.trace.candidates.find((c) => c.connector === connector)!;

describe('expected-cost scoring (PSP A vs PSP B)', () => {
  // amount 100000 so failure_penalty_pct N means L_eff = N * 1000 minor units
  it.each([
    [20, 20000, ['B', 'A'], '4700.0000'],
    [30, 30000, ['A', 'B'], '6000.0000'],
    [50, 50000, ['A', 'B'], '6800.0000'],
  ])('L_eff %s%% -> L=%s picks %j', (pct, lEff, expected, winnerScore) => {
    const d = decide(input(), cfg({ failure_penalty_pct: pct }), AB);
    expect(d.trace.lEff).toBe(String(lEff));
    expect(names(d)).toEqual(expected);
    expect(row(d, expected[0]!).score).toBe(winnerScore);
    expect(d.fallbackToHyperswitch).toBe(false);
  });

  it('reports score components and the break-even penalty', () => {
    const d = decide(input(), cfg({ failure_penalty_pct: 20 }), AB);
    const b = row(d, 'B');
    expect(b.successTerm).toBe('1700.0000'); // 0.85 * 2000
    expect(b.failureTerm).toBe('3000.0000'); // 0.15 * 20000
    expect(b.rank).toBe(1);
    expect(b.selectionReason).toBe('best_score');
    expect(row(d, 'A').selectionReason).toBe('ranked_by_score');
    // L* = (0.96*5000 - 0.85*2000) / 0.11 = 28181.818..., as percent of 100000
    expect(d.trace.breakEvenPenaltyPct).toBeCloseTo(28.1818, 3);
  });

  it('is a pure function of its inputs', () => {
    const c = cfg();
    expect(decide(input(), c, AB)).toEqual(decide(input(), c, AB));
  });
});

describe('L_eff clamp', () => {
  it('clamps up to the highest candidate fee and flags it', () => {
    const d = decide(input(), cfg({ failure_penalty_pct: 1 }), AB); // 1% of 100000 = 1000 < 5000
    expect(d.trace.lEffRaw).toBe('1000');
    expect(d.trace.failurePenaltyPct).toBe(1);
    expect(d.trace.lEff).toBe('5000');
    expect(d.trace.lEffClamped).toBe(true);
  });
  it('does not flag when L_eff already exceeds every fee', () => {
    const d = decide(input(), cfg({ failure_penalty_pct: 20 }), AB);
    expect(d.trace.lEffClamped).toBe(false);
    expect(d.trace.lEffRaw).toBe(d.trace.lEff);
    expect(d.trace.failurePenaltyPct).toBe(20);
  });
  it('penalty 0 leaves L_eff to the clamp alone', () => {
    const d = decide(input(), cfg({ failure_penalty_pct: 0 }), AB);
    expect(d.trace).toMatchObject({ failurePenaltyPct: 0, lEffRaw: '0', lEff: '5000', lEffClamped: true });
  });
  it('penalty 100 means L_eff equals the amount', () => {
    const d = decide(input(), cfg({ failure_penalty_pct: 100 }), AB);
    expect(d.trace).toMatchObject({ failurePenaltyPct: 100, lEffRaw: '100000', lEff: '100000', lEffClamped: false });
  });
  it('never mutates the configured value', () => {
    const c = cfg({ failure_penalty_pct: 1 });
    decide(input(), c, AB);
    expect(c.failurePenaltyPct).toEqual({ num: 1n, den: 1n });
  });
});

describe('tie handling', () => {
  // s_A=0.9 f=3000, s_B=0.8 f=1000: equal expected cost at L = 19000 (19% of 100000)
  const fees = { A: [{ match: { currency: 'INR' }, bps: 0, fixed: 3000 }], B: [{ match: { currency: 'INR' }, bps: 0, fixed: 1000 }] };
  const obs = { A: counts({ success: 90, technical_failure: 10 }), B: counts({ success: 80, technical_failure: 20 }) };

  it('breaks a score tie by higher s_hat regardless of input order', () => {
    const c = cfg({ fee_table: fees, failure_penalty_pct: 19 });
    const fwd = decide(input({ candidates: [cand('A'), cand('B')] }), c, obs);
    const rev = decide(input({ candidates: [cand('B'), cand('A')] }), c, obs);
    expect(row(fwd, 'A').score).toBe(row(fwd, 'B').score);
    expect(names(fwd)).toEqual(['A', 'B']);
    expect(names(rev)).toEqual(['A', 'B']);
    expect(fwd.trace.breakEvenPenaltyPct).toBe(19);
  });

  it('then lower fee, then connector name, then merchantConnectorId', () => {
    const same = [{ match: { currency: 'INR' }, bps: 0, fixed: 100 }];
    const c = cfg({ fee_table: { X: same, Y: same, Z: [{ match: { currency: 'INR' }, bps: 0, fixed: 100 }] } });
    const d = decide(input({ candidates: [cand('Z'), cand('Y'), cand('X', 'mca_2'), cand('X', 'mca_1')] }), c, {});
    expect(d.ordered.map((x) => `${x.connector}/${x.merchantConnectorId}`)).toEqual(['X/mca_1', 'X/mca_2', 'Y/mca_Y', 'Z/mca_Z']);
  });
});

describe('pricing', () => {
  it('zero fee is valid and wins on cost', () => {
    const c = cfg({ fee_table: { A: [{ match: { currency: 'INR' }, bps: 0, fixed: 0 }], B: [{ match: { currency: 'INR' }, bps: 0, fixed: 100 }] } });
    const d = decide(input(), c, {});
    expect(names(d)).toEqual(['A', 'B']);
    expect(row(d, 'A').fee).toBe('0');
  });

  it('ranks unpriced connectors last (by s_hat desc, then name) and flags them', () => {
    const c = cfg();
    const d = decide(
      input({ candidates: [cand('Q'), cand('A'), cand('P'), cand('B')] }),
      c,
      { Q: counts({ success: 5 }), P: counts({ success: 5 }) },
    );
    expect(names(d)).toEqual(['B', 'A', 'P', 'Q']);
    expect(row(d, 'P').status).toBe('unpriced');
    expect(row(d, 'P').selectionReason).toBe('unpriced_last');
    expect(row(d, 'P').fee).toBeNull();
    expect(row(d, 'P').score).toBeNull();
  });

  it('falls back to Hyperswitch with an empty override when nothing is priced', () => {
    const d = decide(input({ currency: 'USD' }), cfg(), AB);
    expect(d.fallbackToHyperswitch).toBe(true);
    expect(d.ordered).toEqual([]);
    expect(d.trace.summary).toMatch(/deferring to Hyperswitch/);
  });

  it('handles amounts near 2^53 without precision loss', () => {
    const c = cfg({ fee_table: { A: [{ match: { currency: 'INR' }, bps: 150, fixed: 0 }], B: [{ match: { currency: 'INR' }, bps: 100, fixed: 0 }] } });
    const d = decide(input({ amountMinor: 9007199254740991n }), c, {});
    expect(row(d, 'A').fee).toBe('135107988821115');
    expect(row(d, 'B').fee).toBe('90071992547410'); // 90071992547409.91 rounded
  });
});

describe('reliability inputs', () => {
  it('gives every candidate the same pooled prior, so a connector with no data takes it', () => {
    const c = cfg({ min_sample_size: 10 });
    const d = decide(input(), c, { B: counts({ success: 50, technical_failure: 50 }) });
    const a = row(d, 'A').sHat; // no data: exactly the pooled prior 50/100
    expect(compareRatio(ratio(BigInt(a.num), BigInt(a.den)), ratio(1n, 2n))).toBe(0);
    const b = row(d, 'B').sHat; // (50 + 10*0.5) / (100 + 10) = 0.5
    expect(compareRatio(ratio(BigInt(b.num), BigInt(b.den)), ratio(1n, 2n))).toBe(0);
  });

  it('does not count unresolved attempts as failures', () => {
    const c = cfg({ min_sample_size: 10 });
    const clean = decide(input(), c, { A: counts({ success: 9, technical_failure: 1 }), B: counts({ success: 8, technical_failure: 2 }) });
    const withPending = decide(
      input(),
      c,
      { A: counts({ success: 9, technical_failure: 1, unresolved: 1000 }), B: counts({ success: 8, technical_failure: 2, unresolved: 1000, ignored: 7 }) },
    );
    expect(row(withPending, 'A').sHat).toEqual(row(clean, 'A').sHat);
    expect(row(withPending, 'B').sHat).toEqual(row(clean, 'B').sHat);
    expect(row(withPending, 'A').observed.unresolved).toBe(1000);
  });

  it('stays deterministic when every connector is failing', () => {
    const allBad = { A: counts({ success: 0, technical_failure: 50 }), B: counts({ success: 0, technical_failure: 50 }) };
    const c = cfg();
    const a = decide(input({ candidates: [cand('A'), cand('B')] }), c, allBad);
    const b = decide(input({ candidates: [cand('B'), cand('A')] }), c, allBad);
    expect(names(a)).toEqual(['B', 'A']); // equal s_hat 0 -> lower score (cheaper fee) first
    expect(names(b)).toEqual(names(a));
    expect(a.fallbackToHyperswitch).toBe(false);
  });
});

describe('min_success_rate floor', () => {
  it('0 excludes nobody', () => {
    const d = decide(input(), cfg({ min_success_rate: 0 }), { A: counts({ success: 1, technical_failure: 99 }), B: counts({ technical_failure: 100 }) });
    expect(d.fallbackToHyperswitch).toBe(false);
    expect(d.ordered).toHaveLength(2);
  });
  it('drops connectors below the floor when at least one passes', () => {
    const d = decide(input(), cfg({ min_success_rate: 0.9 }), AB); // B at 0.85 is under
    expect(names(d)).toEqual(['A']);
    expect(d.fallbackToHyperswitch).toBe(false);
    expect(d.fallbackReason).toBeNull();
    expect(row(d, 'B').status).toBe('below_floor');
    expect(row(d, 'B').statusReason).toMatch(/reliability_floor_unmet.*0\.8500.*0\.9/);
    expect(row(d, 'B').rank).toBeNull();
    expect(row(d, 'B').selectionReason).toBe('excluded');
  });
  it('a candidate exactly at the floor passes', () => {
    expect(names(decide(input(), cfg({ min_success_rate: 0.85 }), AB))).toEqual(['B', 'A']);
  });
  it('falls back to Hyperswitch (no override, no success-rate ranking) when every priced connector is under it', () => {
    const d = decide(input(), cfg({ min_success_rate: 0.99 }), AB);
    expect(d.fallbackToHyperswitch).toBe(true);
    expect(d.ordered).toEqual([]);
    expect(d.fallbackReason).toBe('reliability_floor_unmet');
    expect(d.trace.fallbackReason).toBe('reliability_floor_unmet');
    expect(d.trace.summary).toMatch(/reliability_floor_unmet/);
    expect(row(d, 'A')).toMatchObject({ status: 'below_floor', rank: null });
    expect(row(d, 'B')).toMatchObject({ status: 'below_floor', rank: null });
    expect(row(d, 'A').statusReason).toMatch(/0\.9600/);
  });
  it('still omits the override when unpriced connectors exist but all priced ones fail the floor', () => {
    const d = decide(
      input({ candidates: [cand('A'), cand('B'), cand('U')] }),
      cfg({ min_success_rate: 0.99 }),
      { ...AB, U: counts({ success: 100 }) }, // U is unpriced and perfectly reliable
    );
    expect(d.fallbackToHyperswitch).toBe(true);
    expect(d.ordered).toEqual([]);
    expect(d.fallbackReason).toBe('reliability_floor_unmet');
    expect(row(d, 'U').status).toBe('unpriced');
  });
  it('reports no_usable_pricing separately', () => {
    expect(decide(input({ currency: 'USD' }), cfg(), AB).fallbackReason).toBe('no_usable_pricing');
  });
});

describe('trace', () => {
  it('carries inputs, per-candidate details and a summary, and no long digit runs', () => {
    const d = decide(input({ binClass: 'premium' }), cfg({ failure_penalty_pct: 20 }), AB);
    const t = d.trace;
    expect(t).toMatchObject({ paymentId: 'pay_1', currency: 'INR', amountMinor: '100000', binClass: 'premium', lEff: '20000', fallbackReason: null });
    expect(row(d, 'A')).toMatchObject({
      connector: 'A',
      merchantConnectorId: 'mca_A',
      fee: '5000',
      status: 'eligible',
      statusReason: 'passed all filters',
      successTerm: '4800.0000',
      failureTerm: '800.0000',
      score: '5600.0000',
      rank: 2,
      observed: { success: 96, technical_failure: 4, issuer_decline: 0, unresolved: 0, ignored: 0 },
    });
    expect(row(d, 'A').sHat.value).toBeCloseTo(0.96, 10);
    expect(t.summary).toContain('chose B');
    expect(t.summary).toContain('over A');
    expect(JSON.stringify(t)).not.toMatch(/\d{13,}/);
  });
  it('can be serialised as JSON (no bigint leaks)', () => {
    expect(() => JSON.stringify(decide(input(), cfg(), AB))).not.toThrow();
  });
});
