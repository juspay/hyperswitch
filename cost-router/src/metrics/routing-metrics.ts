import { Counter, Gauge, Histogram, Registry, collectDefaultMetrics } from 'prom-client';
import type { OutcomeClass } from '../engine/outcome';
import type { RoutingDecision } from '../engine/types';
import type { RoutingOverride } from '../payments/decision.store';

/**
 * Prometheus metrics for routing decisions, per connector. Exposed at GET /metrics.
 * Labels use only configured connector names, currencies and fixed enums (never payment ids), so
 * cardinality stays bounded.
 */
export class RoutingMetrics {
  readonly registry = new Registry();

  private readonly decisions = new Counter({
    name: 'cost_router_routing_decisions_total',
    help: 'Routing decisions by outcome: priority override sent, or omitted (fallback reason / engine error)',
    labelNames: ['override', 'fallback_reason'] as const,
    registers: [this.registry],
  });

  private readonly firstChoice = new Counter({
    name: 'cost_router_connector_first_choice_total',
    help: 'How often each connector was ranked first in a priority override',
    labelNames: ['connector', 'currency'] as const,
    registers: [this.registry],
  });

  private readonly candidateStatus = new Counter({
    name: 'cost_router_candidate_evaluations_total',
    help: 'Per-connector evaluation result in each decision (eligible, below_floor, unpriced, ...)',
    labelNames: ['connector', 'status'] as const,
    registers: [this.registry],
  });

  private readonly successEstimate = new Gauge({
    name: 'cost_router_connector_success_estimate',
    help: 'Latest shrunk success-rate estimate (s_hat) per connector and currency',
    labelNames: ['connector', 'currency'] as const,
    registers: [this.registry],
  });

  private readonly attempts = new Counter({
    name: 'cost_router_attempts_total',
    help: 'Payment attempts read back from Hyperswitch and counted, by connector and outcome class',
    labelNames: ['connector', 'outcome'] as const,
    registers: [this.registry],
  });

  private readonly decisionSeconds = new Histogram({
    name: 'cost_router_decision_duration_seconds',
    help: 'Time spent in the routing engine (decide) per payment',
    buckets: [0.0001, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05],
    registers: [this.registry],
  });

  private readonly requestSeconds = new Histogram({
    name: 'cost_router_request_duration_seconds',
    help: 'POST /payments end to end (decision, Hyperswitch create, attempt read-back)',
    labelNames: ['result'] as const,
    buckets: [0.005, 0.01, 0.025, 0.05, 0.1, 0.2, 0.5, 1, 2.5, 5],
    registers: [this.registry],
  });

  constructor(options: { defaultMetrics?: boolean } = {}) {
    if (options.defaultMetrics) collectDefaultMetrics({ register: this.registry, prefix: 'cost_router_' });
  }

  recordDecision(override: RoutingOverride, decision: RoutingDecision | null, currency: string, seconds: number | null): void {
    this.decisions.inc({ override, fallback_reason: decision?.fallbackReason ?? 'none' });
    if (seconds !== null) this.decisionSeconds.observe(seconds);
    if (!decision) return;
    const first = decision.ordered[0];
    if (override === 'priority' && first) this.firstChoice.inc({ connector: first.connector, currency: currency.toUpperCase() });
    for (const c of decision.trace.candidates) {
      this.candidateStatus.inc({ connector: c.connector, status: c.status });
      this.successEstimate.set({ connector: c.connector, currency: currency.toUpperCase() }, c.sHat.value);
    }
  }

  recordAttempt(connector: string, outcome: OutcomeClass): void {
    this.attempts.inc({ connector, outcome });
  }

  recordRequest(result: 'ok' | 'replayed' | 'error', seconds: number): void {
    this.requestSeconds.observe({ result }, seconds);
  }

  contentType(): string {
    return this.registry.contentType;
  }

  render(): Promise<string> {
    return this.registry.metrics();
  }
}
