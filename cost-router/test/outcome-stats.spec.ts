import {
  ATTEMPT_STATUS_TABLE,
  OUTCOME_CLASSES,
  classifyAttempt,
  compareRatio,
  pooledPrior,
  ratio,
  shrunkSuccessRate,
} from '../src/engine';
import { InMemoryAttemptStatsStore } from '../src/stats/attempt-stats.store';
import { counts } from './helpers';

// Every AttemptStatus variant (Rust names, common_enums/src/enums.rs). A new variant
// must be added here AND classified in ATTEMPT_STATUS_TABLE, or this test fails.
const RUST_VARIANTS = [
  'Started', 'AuthenticationFailed', 'RouterDeclined', 'AuthenticationPending', 'AuthenticationSuccessful',
  'Authorized', 'AuthorizationFailed', 'Charged', 'Authorizing', 'CodInitiated', 'Voided', 'VoidedPostCharge',
  'VoidInitiated', 'CaptureInitiated', 'CaptureFailed', 'VoidFailed', 'AutoRefunded', 'PartialCharged',
  'PartiallyAuthorized', 'PartialChargedAndChargeable', 'Unresolved', 'Pending', 'Failure', 'PaymentMethodAwaited',
  'ConfirmationAwaited', 'DeviceDataCollectionPending', 'IntegrityFailure', 'Expired', 'CaptureReview',
];
const snake = (s: string) => s.replace(/([a-z])([A-Z])/g, '$1_$2').toLowerCase();

const classify = (status: string, code: string | null, codes: ReadonlySet<string>) => classifyAttempt(status, code, codes).outcome;

describe('attempt status classification', () => {
  it('classifies every AttemptStatus and nothing else', () => {
    expect(RUST_VARIANTS).toHaveLength(29);
    expect(Object.keys(ATTEMPT_STATUS_TABLE).sort()).toEqual(RUST_VARIANTS.map(snake).sort());
  });
  it('maps every status to a known outcome class', () => {
    for (const status of Object.keys(ATTEMPT_STATUS_TABLE)) {
      expect(OUTCOME_CLASSES).toContain(classify(status, null, new Set()));
      expect(classifyAttempt(status, null, new Set()).warning).toBeNull();
    }
  });
  it('treats success, unresolved and ignored statuses as documented', () => {
    const none = new Set<string>();
    expect(classify('charged', null, none)).toBe('success');
    expect(classify('authorized', null, none)).toBe('success');
    expect(classify('pending', null, none)).toBe('unresolved');
    expect(classify('authentication_pending', null, none)).toBe('unresolved');
    expect(classify('router_declined', null, none)).toBe('ignored');
    expect(classify('voided', null, none)).toBe('ignored');
  });
  it('splits failures by configured issuer-decline codes; unclassified is technical', () => {
    const codes = new Set(['insufficient_funds']);
    expect(classify('failure', 'insufficient_funds', codes)).toBe('issuer_decline');
    expect(classify('authorization_failed', 'insufficient_funds', codes)).toBe('issuer_decline');
    expect(classify('failure', 'timeout', codes)).toBe('technical_failure');
    expect(classify('failure', null, codes)).toBe('technical_failure');
    expect(classify('failure', 'insufficient_funds', new Set())).toBe('technical_failure');
  });
  it('never penalises an unknown status and reports it as a warning', () => {
    const r = classifyAttempt('brand_new_status', null, new Set());
    expect(r.outcome).toBe('ignored');
    expect(r.warning).toMatch(/brand_new_status/);
  });
  it('gives integrity_failure its own class, never success or technical', () => {
    expect(classify('integrity_failure', null, new Set())).toBe('integrity_failure');
    expect(classify('integrity_failure', 'timeout', new Set(['timeout']))).toBe('integrity_failure');
  });
  it('treats authorized as success and capture/void/refund/auth-failure statuses as ignored', () => {
    const none = new Set<string>();
    expect(classify('authorized', null, none)).toBe('success');
    for (const st of ['authentication_failed', 'router_declined', 'capture_failed', 'capture_initiated', 'void_failed', 'void_initiated', 'voided', 'voided_post_charge', 'auto_refunded', 'capture_review']) {
      expect(classify(st, null, none)).toBe('ignored');
    }
  });
});

describe('shrinkage', () => {
  it('computes (successes + k*prior)/(n + k) exactly', () => {
    // 9/10 observed, k=10, prior 1/2 -> (9 + 5)/20 = 7/10
    const s = shrunkSuccessRate(counts({ success: 9, technical_failure: 1 }), 10, ratio(1n, 2n));
    expect(compareRatio(s, ratio(7n, 10n))).toBe(0);
  });
  it('k = 0 gives the raw rate; no data gives the prior', () => {
    const raw = shrunkSuccessRate(counts({ success: 96, technical_failure: 4 }), 0, ratio(1n, 1n));
    expect(compareRatio(raw, ratio(96n, 100n))).toBe(0);
    expect(compareRatio(shrunkSuccessRate(counts({}), 0, ratio(3n, 4n)), ratio(3n, 4n))).toBe(0);
    expect(compareRatio(shrunkSuccessRate(counts({}), 10, ratio(3n, 4n)), ratio(3n, 4n))).toBe(0);
  });
  it('ignores issuer declines, integrity failures, unresolved and ignored in n', () => {
    const noisy = counts({ success: 9, technical_failure: 1, issuer_decline: 500, integrity_failure: 500, unresolved: 500, ignored: 500 });
    const clean = counts({ success: 9, technical_failure: 1 });
    const prior = ratio(1n, 2n);
    expect(compareRatio(shrunkSuccessRate(noisy, 10, prior), shrunkSuccessRate(clean, 10, prior))).toBe(0);
  });
  it('prior is the pooled rate over all candidates, or 1 when none has data', () => {
    const p = pooledPrior([counts({ success: 30, technical_failure: 10 }), counts({ success: 10, technical_failure: 10 }), counts({})]);
    expect(compareRatio(p, ratio(40n, 60n))).toBe(0);
    expect(compareRatio(pooledPrior([counts({}), counts({ unresolved: 9 })]), ratio(1n, 1n))).toBe(0);
    expect(compareRatio(pooledPrior([]), ratio(1n, 1n))).toBe(0);
  });
  it('with a common prior, thin evidence cannot invert the ordering (regression)', () => {
    // A leave-one-out prior ranked the connector with one success BELOW one that had only failed.
    const stripe = counts({ success: 1 });
    const faux = counts({ success: 1, technical_failure: 1 });
    const pretend = counts({ technical_failure: 1 });
    const prior = pooledPrior([stripe, faux, pretend]); // 2 successes / 4 attempts
    expect(compareRatio(prior, ratio(1n, 2n))).toBe(0);
    const [s, f, p] = [stripe, faux, pretend].map((c) => shrunkSuccessRate(c, 5, prior));
    expect(compareRatio(s!, ratio(7n, 12n))).toBe(0);
    expect(compareRatio(f!, ratio(1n, 2n))).toBe(0);
    expect(compareRatio(p!, ratio(5n, 12n))).toBe(0);
    expect(compareRatio(s!, f!)).toBe(1);
    expect(compareRatio(f!, p!)).toBe(1);
  });
});

describe('InMemoryAttemptStatsStore', () => {
  it('counts per (connector, currency) and expires outside the window', () => {
    let now = 0;
    const store = new InMemoryAttemptStatsStore(10, () => now);
    store.record('A', 'inr', 'success');
    store.record('A', 'INR', 'technical_failure');
    store.record('A', 'USD', 'success');
    expect(store.counts('INR', ['A', 'B'])).toEqual({ A: counts({ success: 1, technical_failure: 1 }), B: counts({}) });

    now = 9 * 60_000; // still inside a 10-minute window
    store.record('A', 'INR', 'success');
    expect(store.counts('INR', ['A']).A).toEqual(counts({ success: 2, technical_failure: 1 }));

    now = 10 * 60_000; // the minute-0 bucket has aged out
    expect(store.counts('INR', ['A']).A).toEqual(counts({ success: 1 }));
    now = 100 * 60_000;
    expect(store.counts('INR', ['A']).A).toEqual(counts({}));
  });
  it('rejects an invalid window', () => {
    expect(() => new InMemoryAttemptStatsStore(0, () => 0)).toThrow();
  });
});
