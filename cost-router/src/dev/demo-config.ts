/**
 * cost-router config for the local dummy-connector stack, shared by scripts/demo.mjs and scripts/serve.mjs.
 * The values are ILLUSTRATIVE business assumptions for the demo (see README "Configuration"), not recommendations.
 * The Hyperswitch API key is never part of this object: it is passed as HYPERSWITCH_API_KEY.
 */
export const DEMO_FEES = {
  fauxpay: { bps: 100, fixed: 10 }, // cheap but flaky
  stripe_test: { bps: 290, fixed: 30 }, // dearer but reliable
} as const;
export const DEMO_FAILURE_PENALTY_PCT = 20;
export const DEMO_MIN_SAMPLE_SIZE = 3;
export const DEMO_WINDOW_MINUTES = 60;

export interface DemoConfigInput {
  port: number;
  hyperswitchUrl: string;
  connectorIds: { fauxpay: string; stripe_test: string };
  minSuccessRate: number;
}

/** Returns the config file content (the same shape as config.example.json, without any API key). */
export function buildDemoConfig(input: DemoConfigInput): Record<string, unknown> {
  return {
    port: input.port,
    hyperswitch: { base_url: input.hyperswitchUrl },
    candidates: [
      { connector: 'fauxpay', merchant_connector_id: input.connectorIds.fauxpay },
      { connector: 'stripe_test', merchant_connector_id: input.connectorIds.stripe_test },
    ],
    routing: {
      fee_table: Object.fromEntries(
        Object.entries(DEMO_FEES).map(([connector, f]) => [connector, [{ match: { currency: 'USD' }, bps: f.bps, fixed: f.fixed }]]),
      ),
      failure_penalty_pct: DEMO_FAILURE_PENALTY_PCT,
      min_sample_size: DEMO_MIN_SAMPLE_SIZE,
      window_minutes: DEMO_WINDOW_MINUTES,
      min_success_rate: input.minSuccessRate,
      issuer_decline_codes: [],
    },
  };
}
