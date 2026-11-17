import { emptyCounts, parseEngineConfig, type Candidate, type ClassCounts, type EngineConfig, type RoutingInput } from '../src/engine';

type Raw = Record<string, unknown>;

export function rawConfig(overrides: Raw = {}): Raw {
  return {
    fee_table: {
      A: [{ match: { currency: 'INR' }, bps: 0, fixed: 5000 }],
      B: [{ match: { currency: 'INR' }, bps: 0, fixed: 2000 }],
    },
    failure_penalty_pct: 20,
    min_sample_size: 0,
    window_minutes: 60,
    min_success_rate: 0,
    ...overrides,
  };
}

export function cfg(overrides: Raw = {}): EngineConfig {
  return parseEngineConfig(rawConfig(overrides));
}

export function counts(c: Partial<ClassCounts>): ClassCounts {
  return { ...emptyCounts(), ...c };
}

export const cand = (connector: string, merchantConnectorId = `mca_${connector}`): Candidate => ({ connector, merchantConnectorId });

export function input(over: Partial<RoutingInput> = {}): RoutingInput {
  return { paymentId: 'pay_1', currency: 'INR', amountMinor: 100000n, candidates: [cand('A'), cand('B')], ...over };
}
