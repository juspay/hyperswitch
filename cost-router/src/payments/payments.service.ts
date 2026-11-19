import { performance } from 'node:perf_hooks';
import { BadRequestException, ConflictException, HttpException, Logger } from '@nestjs/common';
import type { AppConfig } from '../app.config';
import { decide } from '../engine/decide';
import { classifyAttempt } from '../engine/outcome';
import type { RoutingDecision } from '../engine/types';
import { buildPriorityRouting, parseAttempts, type AttemptView, type HyperswitchClient } from '../hyperswitch/client';
import { AttemptDedupe } from '../stats/attempt-dedupe';
import type { AttemptStatsStore, Clock } from '../stats/attempt-stats.store';
import type { DecisionStore, RoutingOverride, TraceRecord } from './decision.store';
import type { RoutingMetrics } from '../metrics/routing-metrics';
import { describeExecution } from './execution';
import { requestFingerprint } from './fingerprint';

export interface PaymentHints {
  binClass?: string;
  cardType?: string;
}

export interface PaymentsServiceOptions {
  /** Shared de-duplication memory (so it can be saved and restored); a private one is created when absent. */
  dedupe?: AttemptDedupe;
  /** Called after a decision is stored and after a payment completes, to save the state file. */
  persist?: () => void;
  /** Prometheus metrics (GET /metrics); nothing is recorded when absent. */
  metrics?: RoutingMetrics;
}

export interface PaymentOutcome {
  replayed: boolean;
  record: TraceRecord;
}

/** Unambiguous dedupe key: (payment, attempt, lifecycle stage). JSON avoids separator collisions. */
function attemptKey(paymentId: string, attemptId: string, kind: 'unresolved' | 'terminal'): string {
  return JSON.stringify([paymentId, attemptId, kind]);
}

export class PaymentsService {
  private readonly log = new Logger('cost-router');
  private readonly inflight = new Map<string, { fingerprint: string; run: Promise<PaymentOutcome> }>();
  private readonly dedupe: AttemptDedupe;

  constructor(
    private readonly config: AppConfig,
    private readonly client: HyperswitchClient,
    private readonly stats: AttemptStatsStore,
    private readonly decisions: DecisionStore,
    private readonly clock: Clock,
    private readonly options: PaymentsServiceOptions = {},
  ) {
    this.dedupe =
      options.dedupe ?? new AttemptDedupe(config.engine.attemptDedupeTtlMinutes, config.engine.attemptDedupeMaxEntries, clock);
  }

  /** Writes the optional state file. A write failure is logged, never turned into a failed payment. */
  private persist(): void {
    if (!this.options.persist) return;
    try {
      this.options.persist();
    } catch (e) {
      this.log.error(`state file write failed (statistics are still in memory): ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  /**
   * Concurrent requests with the same payment id and the same request share one execution; the same id
   * with a different request gets 409 (first wins).
   */
  create(paymentId: string, body: Record<string, unknown>, hints: PaymentHints): Promise<PaymentOutcome> {
    const metrics = this.options.metrics;
    if (!metrics) return this.createUntimed(paymentId, body, hints);
    const started = performance.now();
    const seconds = () => (performance.now() - started) / 1000;
    return this.createUntimed(paymentId, body, hints).then(
      (out) => {
        metrics.recordRequest(out.replayed ? 'replayed' : 'ok', seconds());
        return out;
      },
      (e: unknown) => {
        metrics.recordRequest('error', seconds());
        throw e;
      },
    );
  }

  private createUntimed(paymentId: string, body: Record<string, unknown>, hints: PaymentHints): Promise<PaymentOutcome> {
    const fingerprint = requestFingerprint(paymentId, body);
    const running = this.inflight.get(paymentId);
    if (running) {
      if (running.fingerprint !== fingerprint) return Promise.reject(this.conflict());
      return running.run;
    }
    this.decisions.beginRequest(paymentId); // pinned in the store until this request settles
    const run = this.execute(paymentId, body, hints, fingerprint).finally(() => {
      this.inflight.delete(paymentId);
      this.decisions.endRequest(paymentId);
    });
    this.inflight.set(paymentId, { fingerprint, run });
    return run;
  }

  private conflict(): ConflictException {
    return new ConflictException('this payment id was already used with a different request body; use a new Idempotency-Key');
  }

  private async execute(
    paymentId: string,
    body: Record<string, unknown>,
    hints: PaymentHints,
    fingerprint: string,
  ): Promise<PaymentOutcome> {
    const existing = this.decisions.get(paymentId);
    // Fingerprint check comes first: a stored response is never returned for a different request.
    if (existing && existing.fingerprint !== fingerprint) throw this.conflict();
    if (existing?.response) return { replayed: true, record: existing };

    const amount = body['amount'];
    const currency = body['currency'];
    if (typeof amount !== 'number' || !Number.isSafeInteger(amount) || amount < 0) {
      throw new BadRequestException('amount must be a non-negative integer in minor units');
    }
    if (typeof currency !== 'string' || currency.length === 0) {
      throw new BadRequestException('currency is required');
    }

    // A previous forward that failed keeps its stored decision: same payment id, same decision.
    const record = existing ?? this.decideAndStore(paymentId, amount, currency, hints, fingerprint);

    const forwarded: Record<string, unknown> = { ...body, payment_id: paymentId };
    if (record.routingOverride === 'priority' && record.decision) {
      forwarded['routing'] = buildPriorityRouting(record.decision.ordered);
    } else {
      delete forwarded['routing']; // never pass a caller-supplied override through silently
    }

    const response = await this.client.createPayment(forwarded);
    if (response.status < 200 || response.status >= 300) {
      throw new HttpException(response.body, response.status);
    }
    record.response = response;
    const attempts = await this.recordAttempts(paymentId, currency, response.body);
    const requested = record.routingOverride === 'priority' && record.decision ? record.decision.ordered.map((c) => c.connector) : null;
    record.execution = describeExecution(requested, response.body, attempts);
    this.persist();
    return { replayed: false, record };
  }

  private decideAndStore(paymentId: string, amount: number, currency: string, hints: PaymentHints, fingerprint: string): TraceRecord {
    let decision: RoutingDecision | null = null;
    let routingOverride: RoutingOverride;
    let error: string | undefined;
    let decideSeconds: number | null = null;
    try {
      const names = this.config.candidates.map((c) => c.connector);
      const started = performance.now();
      decision = decide(
        {
          paymentId,
          currency,
          amountMinor: BigInt(amount),
          candidates: this.config.candidates,
          ...(hints.binClass !== undefined && { binClass: hints.binClass }),
          ...(hints.cardType !== undefined && { cardType: hints.cardType }),
        },
        this.config.engine,
        this.stats.counts(currency, names),
      );
      decideSeconds = (performance.now() - started) / 1000;
      routingOverride = decision.fallbackToHyperswitch ? 'omitted_fallback' : 'priority';
      this.log.log(decision.trace.summary);
    } catch (e) {
      // A payment must never fail because of this layer: forward without an override.
      routingOverride = 'omitted_error';
      error = e instanceof Error ? e.message : String(e);
      this.log.error(`routing failed for ${paymentId}, override omitted: ${error}`);
    }
    const record: TraceRecord = {
      paymentId,
      createdAtMs: this.clock(),
      fingerprint,
      decision,
      routingOverride,
      ...(error !== undefined && { error }),
    };
    this.decisions.save(record);
    this.options.metrics?.recordDecision(routingOverride, decision, currency, decideSeconds);
    this.persist(); // the decision survives a crash during the forward, so a retry reuses it
    return record;
  }

  /** Per-attempt (not per-payment) accounting: a failed first attempt is charged to its connector. */
  private async recordAttempts(paymentId: string, currency: string, body: Record<string, unknown>): Promise<AttemptView[] | null> {
    let attempts: AttemptView[] | null = parseAttempts(body);
    if (attempts === null) {
      try {
        attempts = await this.client.retrieveAttempts(paymentId);
      } catch (e) {
        this.log.warn(`could not read attempts for ${paymentId}: ${e instanceof Error ? e.message : String(e)}`);
        return null;
      }
    }
    this.ingestAttempts(paymentId, currency, attempts);
    return attempts;
  }

  /**
   * Synchronous on purpose: the dedupe key is claimed before anything can yield, so two
   * interleaved ingestions of the same attempt cannot both count it.
   *
   * Dedupe key = (paymentId, attemptId, stage) where stage is 'unresolved' or 'terminal'. Attempt ids are payment-scoped in Hyperswitch, so the pair is the
   * safe choice. Unresolved sightings use a separate key and never consume the terminal slot: an attempt
   * seen as pending and later terminal is counted once as unresolved and once as its terminal class
   * (first terminal sighting wins; there is no re-resolution or re-classification).
   */
  ingestAttempts(paymentId: string, currency: string, attempts: readonly AttemptView[]): void {
    for (const a of attempts) {
      if (a.attemptId === null) {
        this.log.warn(`attempt without attempt_id on payment ${paymentId} (connector ${a.connector}) cannot be de-duplicated and is not counted`);
        continue;
      }
      const { outcome, warning } = classifyAttempt(a.status, a.errorCode, this.config.engine.issuerDeclineCodes);
      if (warning) this.log.warn(`${warning} (payment ${paymentId}, attempt ${a.attemptId})`);
      const kind = outcome === 'unresolved' ? 'unresolved' : 'terminal';
      // A late or replayed pending sighting after the terminal one is stale: do not count it.
      if (kind === 'unresolved' && this.dedupe.has(attemptKey(paymentId, a.attemptId, 'terminal'))) continue;
      const key = attemptKey(paymentId, a.attemptId, kind);
      if (!this.dedupe.claim(key)) continue;
      this.stats.record(a.connector, currency, outcome);
      this.options.metrics?.recordAttempt(a.connector, outcome);
    }
  }

  getTrace(paymentId: string): TraceRecord | undefined {
    return this.decisions.get(paymentId);
  }
}
