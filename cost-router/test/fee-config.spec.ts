import { ConfigError, calculateFee, parseEngineConfig, resolveFee, type FeeRule } from '../src/engine';
import { cand, cfg, input, rawConfig } from './helpers';

const rule = (over: Partial<FeeRule> = {}): FeeRule => ({
  match: { currency: 'INR' },
  bps: 0n,
  fixed: 0n,
  min: 0n,
  cap: null,
  ...over,
});

describe('calculateFee', () => {
  it('rounds half up', () => {
    expect(calculateFee(rule({ bps: 5000n }), 1n)).toBe(1n); // 0.5 -> 1
    expect(calculateFee(rule({ bps: 5000n }), 3n)).toBe(2n); // 1.5 -> 2
    expect(calculateFee(rule({ bps: 50n }), 10n)).toBe(0n); // 0.05 -> 0
    expect(calculateFee(rule({ bps: 100n }), 10049n)).toBe(100n); // 100.49 -> 100
    expect(calculateFee(rule({ bps: 100n }), 10050n)).toBe(101n); // 100.5 -> 101
  });
  it('adds fixed, then applies min and cap', () => {
    expect(calculateFee(rule({ bps: 200n, fixed: 300n }), 100000n)).toBe(2300n);
    expect(calculateFee(rule({ bps: 200n, min: 5000n }), 100000n)).toBe(5000n);
    expect(calculateFee(rule({ bps: 200n, cap: 1500n }), 100000n)).toBe(1500n);
    expect(calculateFee(rule({ bps: 200n, min: 10n, cap: 10n }), 100000n)).toBe(10n);
  });
  it('allows a zero fee', () => {
    expect(calculateFee(rule(), 123456n)).toBe(0n);
    expect(calculateFee(rule(), 0n)).toBe(0n);
  });
  it('is exact where amount*bps exceeds 2^53', () => {
    // 9007199254740991 * 150 / 10000 = 135107988821114.865
    expect(calculateFee(rule({ bps: 150n }), 9007199254740991n)).toBe(135107988821115n);
    expect(calculateFee(rule({ bps: 10000n }), 9007199254740991n)).toBe(9007199254740991n);
  });
});

describe('fee matching', () => {
  const config = cfg({
    fee_table: {
      A: [
        { match: { currency: 'INR' }, bps: 0, fixed: 100 },
        { match: { currency: 'inr', bin_class: 'premium' }, bps: 0, fixed: 200 },
        { match: { currency: 'INR', bin_class: 'premium', min_amount_minor: 50000 }, bps: 0, fixed: 300 },
      ],
    },
  });
  const rules = config.feeTable.get('A');
  const feeFor = (over: Parameters<typeof input>[0]) => resolveFee(rules, input({ candidates: [cand('A')], ...over }));

  it('picks the most specific rule first', () => {
    expect(feeFor({})).toBe(100n);
    expect(feeFor({ binClass: 'premium', amountMinor: 1000n })).toBe(200n);
    expect(feeFor({ binClass: 'premium', amountMinor: 50000n })).toBe(300n);
  });
  it('returns null when nothing matches (unpriced)', () => {
    expect(feeFor({ currency: 'USD' })).toBeNull();
    expect(resolveFee(undefined, input())).toBeNull();
  });
});

describe('config validation', () => {
  const ruleWith = (over: Record<string, unknown>) => ({ match: { currency: 'INR' }, bps: 0, fixed: 0, ...over });
  const bad = (over: Record<string, unknown>) => () => parseEngineConfig(rawConfig(over));

  it('accepts a valid config', () => {
    expect(() => parseEngineConfig(rawConfig())).not.toThrow();
  });
  it.each([
    ['negative bps', { fee_table: { A: [ruleWith({ bps: -1 })] } }],
    ['negative fixed', { fee_table: { A: [ruleWith({ fixed: -1 })] } }],
    ['non-integer fixed', { fee_table: { A: [ruleWith({ fixed: 1.5 })] } }],
    ['non-integer bps', { fee_table: { A: [ruleWith({ bps: 0.5 })] } }],
    ['bps above 10000', { fee_table: { A: [ruleWith({ bps: 10001 })] } }],
    ['cap < min', { fee_table: { A: [ruleWith({ min: 10, cap: 5 })] } }],
    ['unsafe integer', { fee_table: { A: [ruleWith({ fixed: 2 ** 53 })] } }],
    ['band min > max', { fee_table: { A: [ruleWith({ match: { currency: 'INR', min_amount_minor: 9, max_amount_minor: 1 } })] } }],
    ['duplicate rules', { fee_table: { A: [ruleWith({}), ruleWith({ fixed: 5 })] } }],
    [
      'ambiguous overlapping bands',
      {
        fee_table: {
          A: [
            ruleWith({ match: { currency: 'INR', min_amount_minor: 0, max_amount_minor: 100 } }),
            ruleWith({ match: { currency: 'INR', min_amount_minor: 100, max_amount_minor: 200 } }),
          ],
        },
      },
    ],
    [
      'ambiguous bin_class vs card_type',
      {
        fee_table: {
          A: [
            ruleWith({ match: { currency: 'INR', bin_class: 'x' } }),
            ruleWith({ match: { currency: 'INR', card_type: 'debit' } }),
          ],
        },
      },
    ],
    ['empty rule list', { fee_table: { A: [] } }],
    ['unknown field (typo)', { fee_table: { A: [ruleWith({ fixd: 1 })] } }],
    ['penalty above 100', { failure_penalty_pct: 100.5 }],
    ['floor below 0', { min_success_rate: -0.1 }],
    ['non-finite floor', { min_success_rate: NaN }],
    ['dedupe ttl below window', { window_minutes: 60, attempt_dedupe_ttl_minutes: 59 }],
    ['zero dedupe cap', { attempt_dedupe_max_entries: 0 }],
    ['negative penalty', { failure_penalty_pct: -5 }],
    ['infinite penalty', { failure_penalty_pct: Infinity }],
    ['NaN penalty', { failure_penalty_pct: NaN }],
    ['exponent-notation penalty', { failure_penalty_pct: 1e-7 }],
    ['fractional min_sample_size', { min_sample_size: 1.5 }],
    ['negative min_sample_size', { min_sample_size: -1 }],
    ['zero window', { window_minutes: 0 }],
    ['floor above 1', { min_success_rate: 1.5 }],
  ])('rejects %s', (_name, over) => {
    expect(bad(over)).toThrow(ConfigError);
  });

  it('requires failure_penalty_pct (no default)', () => {
    const raw = rawConfig();
    delete raw['failure_penalty_pct'];
    expect(() => parseEngineConfig(raw)).toThrow(/failure_penalty_pct/);
  });
  it('accepts non-overlapping bands and specific-over-generic overlaps', () => {
    expect(
      bad({
        fee_table: {
          A: [
            ruleWith({ match: { currency: 'INR', max_amount_minor: 99 } }),
            ruleWith({ match: { currency: 'INR', min_amount_minor: 100 } }),
            ruleWith({ match: { currency: 'INR', bin_class: 'x', min_amount_minor: 0 } }),
          ],
        },
      }),
    ).not.toThrow();
  });
  it('requires min_success_rate and allows 0 and 1', () => {
    const raw = rawConfig();
    delete raw['min_success_rate'];
    expect(() => parseEngineConfig(raw)).toThrow(/min_success_rate/);
    expect(bad({ min_success_rate: 0 })).not.toThrow();
    expect(bad({ min_success_rate: 1 })).not.toThrow();
  });
  it('allows a penalty of 0 and 100', () => {
    expect(bad({ failure_penalty_pct: 0 })).not.toThrow();
    expect(bad({ failure_penalty_pct: 100 })).not.toThrow();
  });
  it('defaults issuer_decline_codes to empty and dedupe limits to documented implementation values', () => {
    const c = cfg({ window_minutes: 30 });
    expect(c.issuerDeclineCodes.size).toBe(0);
    expect(c.attemptDedupeTtlMinutes).toBe(60); // 2 x window
    expect(c.attemptDedupeMaxEntries).toBe(100000);
    expect(cfg({ window_minutes: 30, attempt_dedupe_ttl_minutes: 30 }).attemptDedupeTtlMinutes).toBe(30);
  });
});
