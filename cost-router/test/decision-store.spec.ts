import type { INestApplication } from '@nestjs/common';
import { Test } from '@nestjs/testing';
import type { AddressInfo } from 'node:net';
import { parseAppConfig } from '../src/app.config';
import { AppModule } from '../src/app.module';
import type { HyperswitchClient } from '../src/hyperswitch/client';
import { InMemoryDecisionStore, type TraceRecord } from '../src/payments/decision.store';
import { canonicalJson, requestFingerprint } from '../src/payments/fingerprint';
import { PaymentsService } from '../src/payments/payments.service';
import { rawConfig } from './helpers';

const rec = (paymentId: string): TraceRecord => ({
  paymentId,
  createdAtMs: 0,
  fingerprint: 'f',
  decision: null,
  routingOverride: 'omitted_error',
});
const MIN = 60_000;

describe('InMemoryDecisionStore', () => {
  it('expires entries after the TTL', () => {
    let now = 0;
    const s = new InMemoryDecisionStore(10, 100, () => now);
    s.save(rec('a'));
    now = 10 * MIN - 1;
    expect(s.get('a')).toBeDefined();
    now = 10 * MIN;
    expect(s.get('a')).toBeUndefined();
  });
  it('evicts oldest first when over the cap', () => {
    const s = new InMemoryDecisionStore(10, 3, () => 0);
    for (const id of ['a', 'b', 'c', 'd', 'e']) s.save(rec(id));
    expect(['a', 'b', 'c', 'd', 'e'].map((id) => s.get(id) !== undefined)).toEqual([false, false, true, true, true]);
  });
  it('updating an entry keeps its age (it is not refreshed to newest)', () => {
    const s = new InMemoryDecisionStore(10, 2, () => 0);
    s.save(rec('a'));
    s.save(rec('b'));
    s.save(rec('a')); // update
    s.save(rec('c')); // evicts the oldest, which is still a
    expect(s.get('a')).toBeUndefined();
    expect(s.get('b')).toBeDefined();
  });
  it('never evicts or expires an in-flight id', () => {
    let now = 0;
    const s = new InMemoryDecisionStore(10, 2, () => now);
    s.beginRequest('a');
    s.save(rec('a'));
    s.save(rec('b'));
    s.save(rec('c')); // over the cap: oldest is a (in flight), so b goes instead
    expect(s.get('a')).toBeDefined();
    expect(s.get('b')).toBeUndefined();
    now = 100 * MIN; // far past the TTL
    expect(s.get('a')).toBeDefined();
    s.endRequest('a');
    expect(s.get('a')).toBeUndefined(); // now it can expire
  });
  it('counts overlapping requests for the same id', () => {
    let now = 0;
    const s = new InMemoryDecisionStore(1, 10, () => now);
    s.beginRequest('a');
    s.beginRequest('a');
    s.save(rec('a'));
    s.endRequest('a');
    now = 5 * MIN;
    expect(s.get('a')).toBeDefined(); // one request still in flight
    s.endRequest('a');
    expect(s.get('a')).toBeUndefined();
  });
  it('rejects invalid bounds', () => {
    expect(() => new InMemoryDecisionStore(0, 1, () => 0)).toThrow();
    expect(() => new InMemoryDecisionStore(1, 0, () => 0)).toThrow();
  });
});

describe('request fingerprint', () => {
  it('is independent of key order, caller routing and body payment_id; stores only a hash', () => {
    const a = requestFingerprint('p', { amount: 1, nested: { x: 1, y: [1, 2] }, routing: { type: 'single' } });
    const b = requestFingerprint('p', { nested: { y: [1, 2], x: 1 }, payment_id: 'p', amount: 1 });
    expect(a).toBe(b);
    expect(a).toMatch(/^[0-9a-f]{64}$/);
  });
  it('changes with the payment id, values, array order and extra fields', () => {
    const base = requestFingerprint('p', { amount: 1, l: [1, 2] });
    expect(requestFingerprint('q', { amount: 1, l: [1, 2] })).not.toBe(base);
    expect(requestFingerprint('p', { amount: 2, l: [1, 2] })).not.toBe(base);
    expect(requestFingerprint('p', { amount: 1, l: [2, 1] })).not.toBe(base);
    expect(requestFingerprint('p', { amount: 1, l: [1, 2], extra: true })).not.toBe(base);
  });
  it('canonicalises recursively', () => {
    expect(canonicalJson({ b: 1, a: { d: 1, c: [{ z: 1, y: 2 }] } })).toBe('{"a":{"c":[{"y":2,"z":1}],"d":1},"b":1}');
  });
});

describe('decision store inside the service', () => {
  let now = 0;
  let calls: Record<string, unknown>[] = [];
  const client: HyperswitchClient = {
    createPayment: (body) => {
      calls.push(body);
      return Promise.resolve({ status: 200, body: { attempts: [{ attempt_id: `att_${calls.length}`, connector: 'B', status: 'charged' }] } });
    },
    retrieveAttempts: () => Promise.resolve([]),
  };
  const config = (routing: Record<string, unknown> = {}) =>
    parseAppConfig(
      {
        port: 0,
        hyperswitch: { base_url: 'http://unused.invalid', api_key: 'k' },
        candidates: [{ connector: 'A', merchant_connector_id: 'mca_A' }, { connector: 'B', merchant_connector_id: 'mca_B' }],
        routing: rawConfig(routing),
      },
      {},
    );
  beforeEach(() => {
    now = 0;
    calls = [];
  });

  it('an expired id behaves like a new payment (a different body is no longer a conflict)', async () => {
    const cfg = config({ decision_ttl_minutes: 5 });
    const store = new InMemoryDecisionStore(5, 100, () => now);
    const stats = { record: () => undefined, counts: () => ({}) };
    const svc = new PaymentsService(cfg, client, stats, store, () => now);
    const first = await svc.create('p', { amount: 100000, currency: 'INR' }, {});
    expect(first.replayed).toBe(false);
    await expect(svc.create('p', { amount: 5, currency: 'INR' }, {})).rejects.toMatchObject({ status: 409 });
    now = 5 * MIN;
    expect(svc.getTrace('p')).toBeUndefined();
    const again = await svc.create('p', { amount: 5, currency: 'INR' }, {});
    expect(again.replayed).toBe(false);
    expect(calls).toHaveLength(2);
  });

  it('GET routing-trace returns 404 with a clear message for an expired id', async () => {
    const cfg = config({ decision_ttl_minutes: 5 });
    const mod = await Test.createTestingModule({ imports: [AppModule.forRoot({ config: cfg, client, clock: () => now })] }).compile();
    const app: INestApplication = mod.createNestApplication();
    await app.listen(0, '127.0.0.1');
    try {
      const base = `http://127.0.0.1:${(app.getHttpServer().address() as AddressInfo).port}`;
      await fetch(`${base}/payments`, {
        method: 'POST',
        headers: { 'content-type': 'application/json', 'idempotency-key': 'exp' },
        body: JSON.stringify({ amount: 100000, currency: 'INR' }),
      });
      expect((await fetch(`${base}/payments/exp/routing-trace`)).status).toBe(200);
      now = 5 * MIN;
      const gone = await fetch(`${base}/payments/exp/routing-trace`);
      expect(gone.status).toBe(404);
      expect(JSON.stringify(await gone.json())).toMatch(/expired \/ evicted/);
    } finally {
      await app.close();
    }
  });
});
