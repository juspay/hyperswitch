import { parseAppConfig } from '../src/app.config';
import { ConfigError } from '../src/engine/config';
import { DEMO_FAILURE_PENALTY_PCT, DEMO_FEES, DEMO_MIN_SAMPLE_SIZE, buildDemoConfig } from '../src/dev/demo-config';

const input = { port: 3100, hyperswitchUrl: 'http://localhost:8080', connectorIds: { fauxpay: 'mca_f', stripe_test: 'mca_s' }, minSuccessRate: 0 };

describe('buildDemoConfig (used by npm run demo and npm run serve)', () => {
  it('produces a config that passes the real startup validation', () => {
    const app = parseAppConfig(buildDemoConfig(input), { HYPERSWITCH_API_KEY: 'from-env' });
    expect(app.port).toBe(3100);
    expect(app.hyperswitch.baseUrl).toBe('http://localhost:8080');
    expect(app.candidates).toEqual([
      { connector: 'fauxpay', merchantConnectorId: 'mca_f' },
      { connector: 'stripe_test', merchantConnectorId: 'mca_s' },
    ]);
    expect(app.engine.minSampleSize).toBe(DEMO_MIN_SAMPLE_SIZE);
    expect(app.engine.feeTable.get('fauxpay')![0]!.bps).toBe(BigInt(DEMO_FEES.fauxpay.bps));
    expect(app.engine.feeTable.get('stripe_test')![0]!.fixed).toBe(BigInt(DEMO_FEES.stripe_test.fixed));
    expect(app.stateFile).toBeNull();
  });

  it('never contains an API key; the key must come from HYPERSWITCH_API_KEY', () => {
    const cfg = buildDemoConfig(input);
    expect(JSON.stringify(cfg)).not.toMatch(/api_key/);
    expect(() => parseAppConfig(cfg, {})).toThrow(/HYPERSWITCH_API_KEY/);
  });

  it('passes min_success_rate through, and the startup validation still rejects out-of-range values', () => {
    const floor = buildDemoConfig({ ...input, minSuccessRate: 0.85 });
    expect((floor['routing'] as Record<string, unknown>)['min_success_rate']).toBe(0.85);
    expect((floor['routing'] as Record<string, unknown>)['failure_penalty_pct']).toBe(DEMO_FAILURE_PENALTY_PCT);
    expect(() => parseAppConfig(buildDemoConfig({ ...input, minSuccessRate: 1.5 }), { HYPERSWITCH_API_KEY: 'k' })).toThrow(ConfigError);
  });
});
