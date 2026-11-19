import { createServer, type IncomingMessage, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import type { INestApplication } from '@nestjs/common';
import { Test } from '@nestjs/testing';
import { parseAppConfig, type AppConfig } from '../src/app.config';
import { AppModule } from '../src/app.module';
import { HttpHyperswitchClient, buildPriorityRouting } from '../src/hyperswitch/client';
import { PaymentsService } from '../src/payments/payments.service';
import { InMemoryDecisionStore } from '../src/payments/decision.store';
import type { AttemptStatsStore } from '../src/stats/attempt-stats.store';
import { rawConfig } from './helpers';

interface Seen {
  method: string;
  url: string;
  apiKey: string | undefined;
  body: Record<string, unknown>;
}

/** Tiny stand-in for Hyperswitch: records requests, replies with scripted JSON. */
class FakeHyperswitch {
  seen: Seen[] = [];
  createStatus = 200;
  createBody: () => Record<string, unknown> = () => ({});
  retrieveBody: () => Record<string, unknown> = () => ({ attempts: [] });
  private server!: Server;
  baseUrl = '';

  async start(): Promise<void> {
    this.server = createServer((req, res) => void this.handle(req, res));
    await new Promise<void>((r) => this.server.listen(0, '127.0.0.1', r));
    this.baseUrl = `http://127.0.0.1:${(this.server.address() as AddressInfo).port}`;
  }
  stop(): Promise<void> {
    return new Promise((r) => this.server.close(() => r()));
  }
  private async handle(req: IncomingMessage, res: import('node:http').ServerResponse): Promise<void> {
    let raw = '';
    for await (const chunk of req) raw += String(chunk);
    this.seen.push({
      method: req.method ?? '',
      url: req.url ?? '',
      apiKey: req.headers['api-key'] as string | undefined,
      body: raw ? (JSON.parse(raw) as Record<string, unknown>) : {},
    });
    const post = req.method === 'POST';
    res.writeHead(post ? this.createStatus : 200, { 'content-type': 'application/json' });
    res.end(JSON.stringify(post ? this.createBody() : this.retrieveBody()));
  }
  get creates(): Seen[] {
    return this.seen.filter((s) => s.method === 'POST');
  }
}

const attempt = (connector: string, status: string, error_code?: string) => ({
  attempt_id: `att_${connector}_${status}`,
  connector,
  status,
  ...(error_code && { error_code }),
});

function appConfig(baseUrl: string, routing: Record<string, unknown> = {}): AppConfig {
  return parseAppConfig({
    port: 0,
    hyperswitch: { base_url: baseUrl, api_key: 'test-key' },
    candidates: [
      { connector: 'A', merchant_connector_id: 'mca_A' },
      { connector: 'B', merchant_connector_id: 'mca_B' },
    ],
    routing: rawConfig({ issuer_decline_codes: ['insufficient_funds'], ...routing }),
  }, {});
}

const post = (app: INestApplication, path: string, body: unknown, headers: Record<string, string> = {}) =>
  fetch(`${baseUrl(app)}${path}`, { method: 'POST', headers: { 'content-type': 'application/json', ...headers }, body: JSON.stringify(body) });
const baseUrl = (app: INestApplication) => `http://127.0.0.1:${(app.getHttpServer().address() as AddressInfo).port}`;

describe('cost-router adapter against a fake Hyperswitch', () => {
  let hs: FakeHyperswitch;
  let app: INestApplication;

  beforeEach(async () => {
    hs = new FakeHyperswitch();
    await hs.start();
  });
  afterEach(async () => {
    await app?.close();
    await hs.stop();
  });

  async function boot(routing: Record<string, unknown> = {}): Promise<void> {
    const mod = await Test.createTestingModule({
      imports: [AppModule.forRoot({ config: appConfig(hs.baseUrl, routing), clock: () => 0 })],
    }).compile();
    app = mod.createNestApplication();
    await app.listen(0, '127.0.0.1');
  }

  const payBody = { amount: 100000, currency: 'INR', confirm: true };
  const traceOf = async (id: string) => (await fetch(`${baseUrl(app)}/payments/${id}/routing-trace`)).json() as Promise<any>;
  const observed = (t: any, connector: string) => t.decision.trace.candidates.find((c: any) => c.connector === connector).observed;

  it('builds the priority body in one shape and forwards the api-key', async () => {
    expect(buildPriorityRouting([{ connector: 'B', merchantConnectorId: 'mca_B' }])).toEqual({
      type: 'priority',
      data: [{ connector: 'B', merchant_connector_id: 'mca_B' }],
    });
    await boot({ failure_penalty_pct: 20 });
    hs.createBody = () => ({ status: 'succeeded', attempts: [attempt('B', 'charged')] });
    const res = await post(app, '/payments', payBody, { 'idempotency-key': 'pay_1' });
    expect(res.status).toBe(200);
    expect(hs.creates).toHaveLength(1);
    const sent = hs.creates[0]!;
    expect(sent.apiKey).toBe('test-key');
    expect(sent.url).toBe('/payments');
    expect(sent.body['payment_id']).toBe('pay_1');
    expect(sent.body['amount']).toBe(100000);
    // equal data -> cheaper B first (A fee 5000, B fee 2000, both s_hat 1 via the no-data prior)
    expect(sent.body['routing']).toEqual({
      type: 'priority',
      data: [
        { connector: 'B', merchant_connector_id: 'mca_B' },
        { connector: 'A', merchant_connector_id: 'mca_A' },
      ],
    });
  });

  it('stores what Hyperswitch actually did next to the requested order', async () => {
    await boot({ failure_penalty_pct: 20 });
    // We ask for [B, A]; Hyperswitch fails B then succeeds on A, and the response says so.
    hs.createBody = () => ({
      status: 'succeeded',
      connector: 'A',
      attempts: [attempt('A', 'charged'), attempt('B', 'failure', 'timeout')],
    });
    const res = await post(app, '/payments', payBody, { 'idempotency-key': 'exec_1' });
    const out = (await res.json()) as any;
    expect(out.routing.execution.requestedOrder).toEqual(['B', 'A']);
    expect(out.routing.execution.finalConnector).toBe('A');
    expect(out.routing.execution.finalIsFirstRequested).toBe(false);
    expect(out.routing.execution.attemptOrderEstablished).toBe(false);
    const stored = await traceOf('exec_1');
    expect(stored.execution.attempts.map((a: any) => a.connector)).toEqual(['A', 'B']);
    expect(stored.decision.ordered.map((c: any) => c.connector)).toEqual(['B', 'A']); // the request is untouched
  });

  it('exposes per-connector routing metrics in Prometheus format at GET /metrics', async () => {
    await boot({ failure_penalty_pct: 20 });
    hs.createBody = () => ({ status: 'succeeded', connector: 'A', attempts: [attempt('A', 'charged'), attempt('B', 'failure', 'timeout')] });
    await post(app, '/payments', payBody, { 'idempotency-key': 'm_1' });
    const res = await fetch(`${baseUrl(app)}/metrics`);
    expect(res.status).toBe(200);
    expect(res.headers.get('content-type')).toMatch(/text\/plain/);
    const text = await res.text();
    expect(text).toContain('cost_router_routing_decisions_total{override="priority",fallback_reason="none"} 1');
    expect(text).toContain('cost_router_connector_first_choice_total{connector="B",currency="INR"} 1'); // B is cheaper
    expect(text).toContain('cost_router_attempts_total{connector="A",outcome="success"} 1');
    expect(text).toContain('cost_router_attempts_total{connector="B",outcome="technical_failure"} 1');
    expect(text).toMatch(/cost_router_connector_success_estimate\{connector="A",currency="INR"\} 1/);
    expect(text).toMatch(/cost_router_decision_duration_seconds_count 1/);
    expect(text).toMatch(/cost_router_request_duration_seconds_count\{result="ok"\} 1/);
    expect(text).not.toContain('m_1'); // no payment ids as labels
  });

  it('requires a stable payment id and validates the amount', async () => {
    await boot();
    expect((await post(app, '/payments', payBody)).status).toBe(400);
    expect((await post(app, '/payments', { ...payBody, amount: 10.5 }, { 'idempotency-key': 'x' })).status).toBe(400);
    expect((await post(app, '/payments', { ...payBody, payment_id: 'a' }, { 'idempotency-key': 'b' })).status).toBe(400);
    expect(hs.creates).toHaveLength(0);
    // body payment_id alone is enough
    hs.createBody = () => ({ attempts: [attempt('B', 'charged')] });
    expect((await post(app, '/payments', { ...payBody, payment_id: 'from_body' })).status).toBe(200);
  });

  it('credits the failed connector and the successful fallback (per attempt)', async () => {
    await boot();
    hs.createBody = () => ({ status: 'succeeded', attempts: [attempt('B', 'failure', 'timeout'), attempt('A', 'charged')] });
    await post(app, '/payments', payBody, { 'idempotency-key': 'p1' });
    await post(app, '/payments', payBody, { 'idempotency-key': 'p2' }); // reads the stats the first one wrote
    const t = await traceOf('p2');
    expect(observed(t, 'B')).toMatchObject({ technical_failure: 1, success: 0 });
    expect(observed(t, 'A')).toMatchObject({ success: 1, technical_failure: 0 });
  });

  it('records all-fail attempts, splitting configured issuer declines, and ignores unresolved', async () => {
    await boot();
    hs.createBody = () => ({
      status: 'failed',
      attempts: [attempt('B', 'failure', 'timeout'), attempt('A', 'failure', 'insufficient_funds'), attempt('A', 'pending')],
    });
    expect((await post(app, '/payments', payBody, { 'idempotency-key': 'f1' })).status).toBe(200);
    await post(app, '/payments', payBody, { 'idempotency-key': 'f2' });
    const t = await traceOf('f2');
    expect(observed(t, 'B')).toMatchObject({ technical_failure: 1, success: 0 });
    expect(observed(t, 'A')).toMatchObject({ issuer_decline: 1, technical_failure: 0, unresolved: 1 });
  });

  it('falls back to a retrieve with expand_attempts when the create response has no attempts', async () => {
    await boot();
    hs.createBody = () => ({ status: 'succeeded' });
    hs.retrieveBody = () => ({ attempts: [attempt('A', 'charged')] });
    await post(app, '/payments', payBody, { 'idempotency-key': 'r1' });
    const get = hs.seen.find((s) => s.method === 'GET')!;
    expect(get.url).toBe('/payments/r1?expand_attempts=true');
    await post(app, '/payments', payBody, { 'idempotency-key': 'r2' });
    expect(observed(await traceOf('r2'), 'A')).toMatchObject({ success: 1 });
  });

  it('replays the same decision and does not forward again', async () => {
    await boot();
    hs.createBody = () => ({ status: 'succeeded', attempts: [attempt('B', 'charged')] });
    const first = (await (await post(app, '/payments', payBody, { 'idempotency-key': 'same' })).json()) as any;
    const second = (await (await post(app, '/payments', payBody, { 'idempotency-key': 'same' })).json()) as any;
    expect(first.replayed).toBe(false);
    expect(second.replayed).toBe(true);
    expect(second.routing).toEqual(first.routing);
    expect(second.payment).toEqual(first.payment);
    expect(hs.creates).toHaveLength(1);
    expect((await traceOf('same')).decision.trace.summary).toBe(first.routing.decision.trace.summary);
    expect((await fetch(`${baseUrl(app)}/payments/nope/routing-trace`)).status).toBe(404);
    // The path named in the assignment (/v1/...) serves the same record.
    const v1 = await fetch(`${baseUrl(app)}/v1/payments/same/routing-trace`);
    expect(v1.status).toBe(200);
    expect(await v1.json()).toEqual(await traceOf('same'));
    expect((await fetch(`${baseUrl(app)}/v1/payments/nope/routing-trace`)).status).toBe(404);
  });

  it('shares one execution between concurrent requests with the same id', async () => {
    await boot();
    hs.createBody = () => ({ attempts: [attempt('B', 'charged')] });
    await Promise.all([1, 2, 3].map(() => post(app, '/payments', payBody, { 'idempotency-key': 'conc' })));
    expect(hs.creates).toHaveLength(1);
  });

  it('omits the override when no connector has pricing', async () => {
    await boot();
    hs.createBody = () => ({ attempts: [attempt('B', 'charged')] });
    // currency without fee rules -> fallbackToHyperswitch
    const res = (await (await post(app, '/payments', { ...payBody, currency: 'USD', routing: { type: 'single' } }, { 'idempotency-key': 'np' })).json()) as any;
    expect(res.routing.routingOverride).toBe('omitted_fallback');
    expect(hs.creates[0]!.body).not.toHaveProperty('routing');
  });

  describe('request fingerprint', () => {
    const send = (id: string, body: unknown) => post(app, '/payments', body, { 'idempotency-key': id });
    beforeEach(async () => {
      await boot();
      hs.createBody = () => ({ attempts: [attempt('B', 'charged')] });
    });

    it('same body replays, even with different field order and a different caller routing', async () => {
      const first = await send('fp1', { amount: 100000, currency: 'INR', confirm: true, routing: { type: 'single' } });
      const again = await send('fp1', { confirm: true, currency: 'INR', routing: { type: 'priority', data: [] }, amount: 100000 });
      expect(first.status).toBe(200);
      expect(again.status).toBe(200);
      expect(((await again.json()) as any).replayed).toBe(true);
      expect(hs.creates).toHaveLength(1);
    });
    it('different amount -> 409 and no stored data leaks', async () => {
      await send('fp2', payBody);
      const res = await send('fp2', { ...payBody, amount: 100001 });
      expect(res.status).toBe(409);
      const text = JSON.stringify(await res.json());
      expect(text).toMatch(/different request/);
      expect(text).not.toMatch(/decision|attempts|succeeded|mca_/);
      expect(hs.creates).toHaveLength(1);
    });
    it('different currency -> 409', async () => {
      await send('fp3', payBody);
      expect((await send('fp3', { ...payBody, currency: 'USD' })).status).toBe(409);
    });
    it('concurrent requests with the same id but different bodies: first wins, second gets 409', async () => {
      const [a, b] = await Promise.all([send('fp4', payBody), send('fp4', { ...payBody, amount: 5 })]);
      // which request reaches the server first is up to the network; whichever does wins
      expect([a.status, b.status].sort()).toEqual([200, 409]);
      expect(hs.creates).toHaveLength(1);
      expect(hs.creates[0]!.body['amount']).toBe(a.status === 200 ? 100000 : 5);
    });
    it('the stored trace does not contain the request body or its hash', async () => {
      await send('fp5', { ...payBody, payment_method_data: { card: { card_number: '4242424242424242' } } });
      const stored = JSON.stringify(await traceOf('fp5'));
      expect(stored).not.toMatch(/4242|card_number|fingerprint/);
    });
  });

  it('sends no routing field when the reliability floor is unmet or nothing is priced', async () => {
    await boot({ min_success_rate: 0.5 });
    // first payment: both connectors fail, so both have s_hat 0 (k = 0)
    hs.createBody = () => ({ attempts: [attempt('A', 'failure', 'timeout'), attempt('B', 'failure', 'timeout')] });
    await post(app, '/payments', payBody, { 'idempotency-key': 'fl1' });
    expect(hs.creates[0]!.body).toHaveProperty('routing'); // no data yet: prior 1, floor passes
    const res = (await (await post(app, '/payments', { ...payBody, routing: { type: 'single' } }, { 'idempotency-key': 'fl2' })).json()) as any;
    expect(res.routing.routingOverride).toBe('omitted_fallback');
    expect(res.routing.decision.fallbackReason).toBe('reliability_floor_unmet');
    expect(hs.creates[1]!.body).not.toHaveProperty('routing');

    const np = (await (await post(app, '/payments', { ...payBody, currency: 'USD' }, { 'idempotency-key': 'fl3' })).json()) as any;
    expect(np.routing.decision.fallbackReason).toBe('no_usable_pricing');
    expect(hs.creates[2]!.body).not.toHaveProperty('routing');
  });

  it('passes an upstream error through and keeps the decision for a retry with the same id', async () => {
    await boot();
    hs.createStatus = 422;
    hs.createBody = () => ({ error: { message: 'bad' } });
    const res = await post(app, '/payments', payBody, { 'idempotency-key': 'e1' });
    expect(res.status).toBe(422);
    const stored = await traceOf('e1');
    hs.createStatus = 200;
    hs.createBody = () => ({ attempts: [attempt('B', 'charged')] });
    const retry = (await (await post(app, '/payments', payBody, { 'idempotency-key': 'e1' })).json()) as any;
    expect(retry.routing.decision).toEqual(stored.decision);
  });
});

describe('engine or config failure', () => {
  it('omits the routing override and still forwards the payment', async () => {
    const hs = new FakeHyperswitch();
    await hs.start();
    try {
      hs.createBody = () => ({ attempts: [attempt('A', 'charged')] });
      const brokenStats: AttemptStatsStore = {
        record: () => undefined,
        counts: () => {
          throw new Error('stats backend down');
        },
      };
      const service = new PaymentsService(
        appConfig(hs.baseUrl),
        new HttpHyperswitchClient({ baseUrl: hs.baseUrl, apiKey: 'k' }),
        brokenStats,
        new InMemoryDecisionStore(60, 100, () => 0),
        () => 0,
      );
      const { record } = await service.create('boom', { amount: 100000, currency: 'INR', routing: { type: 'single' } }, {});
      expect(record.routingOverride).toBe('omitted_error');
      expect(record.decision).toBeNull();
      expect(record.error).toMatch(/stats backend down/);
      expect(hs.creates).toHaveLength(1);
      expect(hs.creates[0]!.body).not.toHaveProperty('routing');
    } finally {
      await hs.stop();
    }
  });
});
