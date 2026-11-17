import { Logger } from '@nestjs/common';
import { parseAppConfig } from '../src/app.config';
import type { AttemptView, HyperswitchClient } from '../src/hyperswitch/client';
import { PaymentsService } from '../src/payments/payments.service';
import { InMemoryDecisionStore } from '../src/payments/decision.store';
import { AttemptDedupe } from '../src/stats/attempt-dedupe';
import { InMemoryAttemptStatsStore } from '../src/stats/attempt-stats.store';
import { counts, rawConfig } from './helpers';

describe('AttemptDedupe', () => {
  it('claims a key once', () => {
    const d = new AttemptDedupe(10, 100, () => 0);
    expect(d.claim('k')).toBe(true);
    expect(d.claim('k')).toBe(false);
    expect(d.claim('other')).toBe(true);
  });
  it('forgets a key after the TTL', () => {
    let now = 0;
    const d = new AttemptDedupe(10, 100, () => now);
    d.claim('k');
    now = 10 * 60_000 - 1;
    expect(d.claim('k')).toBe(false);
    now = 10 * 60_000;
    expect(d.claim('k')).toBe(true);
    expect(d.size).toBe(1);
  });
  it('evicts oldest first when the cap is exceeded', () => {
    const d = new AttemptDedupe(10, 2, () => 0);
    d.claim('a');
    d.claim('b');
    d.claim('c'); // evicts a
    expect(d.size).toBe(2);
    expect(d.claim('b')).toBe(false);
    expect(d.claim('c')).toBe(false);
    expect(d.claim('a')).toBe(true); // was evicted, so it counts again (documented limit)
  });
});

describe('attempt ingestion', () => {
  let now = 0;
  const attempt = (attemptId: string | null, connector: string, status: string, errorCode: string | null = null): AttemptView => ({
    attemptId,
    connector,
    status,
    errorCode,
  });
  const setup = (routing: Record<string, unknown> = {}) => {
    now = 0;
    const config = parseAppConfig({
      port: 0,
      hyperswitch: { base_url: 'http://unused.invalid', api_key: 'k' },
      candidates: [{ connector: 'A', merchant_connector_id: 'mca_A' }],
      routing: rawConfig(routing),
    }, {});
    const client: HyperswitchClient = {
      createPayment: () => Promise.reject(new Error('unused')),
      retrieveAttempts: () => Promise.reject(new Error('unused')),
    };
    const stats = new InMemoryAttemptStatsStore(config.engine.windowMinutes, () => now);
    const service = new PaymentsService(config, client, stats, new InMemoryDecisionStore(60, 100, () => now), () => now);
    return { service, read: () => stats.counts('INR', ['A']).A! };
  };
  let warn: jest.SpyInstance;
  beforeEach(() => {
    warn = jest.spyOn(Logger.prototype, 'warn').mockImplementation(() => undefined);
  });
  afterEach(() => warn.mockRestore());

  it('does not count a replayed success twice', () => {
    const { service, read } = setup();
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged')]);
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged')]);
    expect(read()).toEqual(counts({ success: 1 }));
  });
  it('does not count a replayed failure twice', () => {
    const { service, read } = setup();
    const a = attempt('a1', 'A', 'failure', 'timeout');
    service.ingestAttempts('p1', 'INR', [a, a]);
    service.ingestAttempts('p1', 'INR', [a]);
    expect(read()).toEqual(counts({ technical_failure: 1 }));
  });
  it('keys on (paymentId, attemptId): the same attempt id on another payment counts', () => {
    const { service, read } = setup();
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged')]);
    service.ingestAttempts('p2', 'INR', [attempt('a1', 'A', 'charged')]);
    expect(read().success).toBe(2);
  });
  it('counts concurrent duplicate ingestions once', async () => {
    const { service, read } = setup();
    const batch = [attempt('a1', 'A', 'charged'), attempt('a2', 'A', 'failure')];
    await Promise.all([1, 2, 3, 4].map(async () => service.ingestAttempts('p1', 'INR', batch)));
    expect(read()).toEqual(counts({ success: 1, technical_failure: 1 }));
  });
  it('counts again after the dedupe TTL has expired', () => {
    const { service, read } = setup({ window_minutes: 60, attempt_dedupe_ttl_minutes: 60 });
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged')]);
    now = 60 * 60_000; // also ages the first sighting out of the stats window
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged')]);
    expect(read().success).toBe(1);
  });
  it('evicts the oldest key when the cap is reached', () => {
    const { service, read } = setup({ attempt_dedupe_max_entries: 2 });
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged'), attempt('a2', 'A', 'charged'), attempt('a3', 'A', 'charged')]);
    service.ingestAttempts('p1', 'INR', [attempt('a3', 'A', 'charged')]); // still remembered
    expect(read().success).toBe(3);
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged')]); // evicted -> counted again
    expect(read().success).toBe(4);
  });
  it('does not count attempts without attempt_id and warns', () => {
    const { service, read } = setup();
    service.ingestAttempts('p1', 'INR', [attempt(null, 'A', 'charged')]);
    expect(read()).toEqual(counts({}));
    expect(warn).toHaveBeenCalledWith(expect.stringMatching(/without attempt_id.*p1/));
  });
  it('an unresolved sighting does not consume the slot for a later terminal one', () => {
    const { service, read } = setup();
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'pending')]);
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'pending')]); // repeat unresolved: not recounted
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged')]);
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'failure')]); // first terminal sighting wins
    expect(read()).toEqual(counts({ unresolved: 1, success: 1 }));
  });
  describe('attempt lifecycle', () => {
    it('Pending -> Charged -> Charged: one unresolved, one success', () => {
      const { service, read } = setup();
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'pending')]);
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged')]);
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged')]);
      expect(read()).toEqual(counts({ unresolved: 1, success: 1 }));
    });
    it('Pending -> Failure -> Failure: one unresolved, one technical failure', () => {
      const { service, read } = setup();
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'pending')]);
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'failure', 'timeout')]);
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'failure', 'timeout')]);
      expect(read()).toEqual(counts({ unresolved: 1, technical_failure: 1 }));
    });
    it('Pending -> Failure with a configured issuer decline code: one unresolved, one issuer decline', () => {
      const { service, read } = setup({ issuer_decline_codes: ['51'] });
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'pending')]);
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'failure', '51')]);
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'failure', '51')]);
      expect(read()).toEqual(counts({ unresolved: 1, issuer_decline: 1 }));
    });
    it('the first terminal sighting wins over a conflicting later one', () => {
      const { service, read } = setup();
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'failure', 'timeout')]);
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged')]);
      expect(read()).toEqual(counts({ technical_failure: 1 }));
    });
    it('a stale pending sighting after the terminal one is not counted', () => {
      const { service, read } = setup();
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'charged')]);
      service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'pending')]);
      expect(read()).toEqual(counts({ success: 1 }));
    });
    it('concurrent interleaved duplicate terminal events count once', async () => {
      for (const terminal of [attempt('a1', 'A', 'charged'), attempt('a1', 'A', 'failure', 'timeout')]) {
        const { service, read } = setup();
        const sequence = [attempt('a1', 'A', 'pending'), terminal, terminal];
        // Four workers advance in lockstep, yielding between events, so their events interleave.
        await Promise.all(
          [1, 2, 3, 4].map(async () => {
            for (const event of sequence) {
              service.ingestAttempts('p1', 'INR', [event]);
              await Promise.resolve();
            }
          }),
        );
        expect(read()).toEqual(
          terminal.status === 'charged' ? counts({ unresolved: 1, success: 1 }) : counts({ unresolved: 1, technical_failure: 1 }),
        );
      }
    });
    it('every arrival order of pending + duplicate terminal events counts the terminal exactly once', () => {
      const events = ['pending', 'terminal', 'terminal'] as const;
      const orders = [
        [0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0],
      ];
      for (const order of orders) {
        const { service, read } = setup();
        for (const i of order) {
          const kind = events[i]!;
          service.ingestAttempts('p1', 'INR', [kind === 'pending' ? attempt('a1', 'A', 'pending') : attempt('a1', 'A', 'charged')]);
        }
        const c = read();
        expect(c.success).toBe(1);
        expect(c.unresolved).toBeLessThanOrEqual(1);
      }
    });
    it('ids containing separator-like characters do not collide', () => {
      const { service, read } = setup();
      service.ingestAttempts('x\u0000y', 'INR', [attempt('z', 'A', 'charged')]);
      service.ingestAttempts('x', 'INR', [attempt('y\u0000z', 'A', 'charged')]);
      service.ingestAttempts('p", "q', 'INR', [attempt('a', 'A', 'charged')]);
      service.ingestAttempts('p', 'INR', [attempt('q", "a', 'A', 'charged')]);
      expect(read().success).toBe(4);
    });
  });
  it('warns on an unknown status and does not count it as a failure', () => {
    const { service, read } = setup();
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'brand_new_status')]);
    expect(read()).toEqual(counts({ ignored: 1 }));
    expect(warn).toHaveBeenCalledWith(expect.stringMatching(/unknown attempt status "brand_new_status"/));
  });
  it('records integrity failures in their own class', () => {
    const { service, read } = setup();
    service.ingestAttempts('p1', 'INR', [attempt('a1', 'A', 'integrity_failure')]);
    expect(read()).toEqual(counts({ integrity_failure: 1 }));
  });
});
