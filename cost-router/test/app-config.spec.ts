import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { ConfigError } from '../src/engine';
import { loadAppConfig, parseAppConfig } from '../src/app.config';
import { rawConfig } from './helpers';

const SECRET = 'sk_live_SUPER_SECRET_VALUE';
const file = (over: Record<string, unknown> = {}) => ({
  port: 3100,
  hyperswitch: { base_url: 'http://file-host:8080' },
  candidates: [{ connector: 'A', merchant_connector_id: 'mca_A' }],
  routing: rawConfig(),
  ...over,
});
const issuesOf = (fn: () => unknown): string[] => {
  try {
    fn();
  } catch (e) {
    expect(e).toBeInstanceOf(ConfigError);
    return (e as ConfigError).issues as string[];
  }
  throw new Error('expected ConfigError');
};

describe('env and file precedence', () => {
  it('reads the api key from the environment', () => {
    const c = parseAppConfig(file(), { HYPERSWITCH_API_KEY: 'env-key' });
    expect(c.hyperswitch).toEqual({ baseUrl: 'http://file-host:8080', apiKey: 'env-key' });
    expect(c.port).toBe(3100);
  });
  it('env overrides file values', () => {
    const c = parseAppConfig(
      file({ hyperswitch: { base_url: 'http://file-host:8080', api_key: 'file-key' }, port: 1 }),
      { HYPERSWITCH_API_KEY: 'env-key', HYPERSWITCH_BASE_URL: 'http://env-host:9', PORT: '4000' },
    );
    expect(c.hyperswitch).toEqual({ baseUrl: 'http://env-host:9', apiKey: 'env-key' });
    expect(c.port).toBe(4000);
  });
  it('falls back to the file key when no env key is set, and works with PORT=0', () => {
    const c = parseAppConfig(file({ hyperswitch: { base_url: 'http://h', api_key: 'file-key' } }), { PORT: '0' });
    expect(c.hyperswitch.apiKey).toBe('file-key');
    expect(c.port).toBe(0);
  });
});

describe('startup errors', () => {
  it('lists every problem in one ConfigError, without echoing the key', () => {
    const raw = file({
      hyperswitch: { api_key: SECRET.slice(0, 0) },
      port: undefined,
      routing: { fee_table: { A: [{ match: { currency: 'INR' }, bps: 0, fixed: 0 }] } },
    });
    const issues = issuesOf(() => parseAppConfig(raw, { PORT: 'abc', HYPERSWITCH_BASE_URL: 'not a url' }));
    const text = issues.join('\n');
    expect(text).toMatch(/api key missing: set HYPERSWITCH_API_KEY/);
    expect(text).toMatch(/base url invalid/);
    expect(text).toMatch(/port invalid/);
    expect(text).toMatch(/failure_penalty_pct is required \(no default\)/);
    expect(text).toMatch(/min_success_rate is required \(no default\)/);
    expect(text).toMatch(/min_sample_size is required \(no default\)/);
    expect(text).toMatch(/window_minutes is required \(no default\)/);
  });
  it('reports missing base url and port', () => {
    const text = issuesOf(() => parseAppConfig({ candidates: file().candidates, routing: rawConfig() }, {})).join('\n');
    expect(text).toMatch(/api key missing/);
    expect(text).toMatch(/base url missing/);
    expect(text).toMatch(/port missing/);
  });
  it('never puts the key in an error message', () => {
    const issues = issuesOf(() =>
      parseAppConfig(file({ routing: { nonsense: true }, candidates: [] }), { HYPERSWITCH_API_KEY: SECRET, PORT: '99999' }),
    );
    expect(issues.length).toBeGreaterThan(1);
    expect(issues.join('\n')).not.toContain(SECRET);
  });
});

describe('config file loading', () => {
  const dir = mkdtempSync(join(tmpdir(), 'cost-router-'));
  it('reports a missing file clearly', () => {
    const missing = join(dir, 'nope.json');
    const issues = issuesOf(() => loadAppConfig(missing, {}));
    expect(issues).toEqual([`config file not found: ${missing} (set COST_ROUTER_CONFIG to its path)`]);
  });
  it('reports invalid JSON without echoing file content', () => {
    const p = join(dir, 'bad.json');
    writeFileSync(p, `{"hyperswitch":{"api_key":"${SECRET}"} oops`);
    const issues = issuesOf(() => loadAppConfig(p, {}));
    expect(issues[0]).toMatch(/^config file is not valid JSON: /);
    expect(issues.join('\n')).not.toContain(SECRET);
  });
  it('loads a valid file with env supplying the key', () => {
    const p = join(dir, 'ok.json');
    writeFileSync(p, JSON.stringify(file()));
    expect(loadAppConfig(p, { HYPERSWITCH_API_KEY: 'k' }).hyperswitch.apiKey).toBe('k');
  });
  it('config.example.json holds no api key and loads once the env supplies one', () => {
    const text = readFileSync(join(__dirname, '..', 'config.example.json'), 'utf8');
    expect(text).not.toMatch(/api_key|REPLACE_ME_key/i);
    const c = loadAppConfig(join(__dirname, '..', 'config.example.json'), { HYPERSWITCH_API_KEY: 'k' });
    expect(c.candidates).toHaveLength(2);
    expect(() => loadAppConfig(join(__dirname, '..', 'config.example.json'), {})).toThrow(/api key missing/);
  });
});

describe('decision store config', () => {
  it('has documented implementation-limit defaults and validates them', () => {
    const c = parseAppConfig(file(), { HYPERSWITCH_API_KEY: 'k' });
    expect(c.engine.decisionTtlMinutes).toBe(1440);
    expect(c.engine.decisionMaxEntries).toBe(100000);
    const bad = issuesOf(() =>
      parseAppConfig(file({ routing: rawConfig({ decision_ttl_minutes: 0, decision_max_entries: 1.5 }) }), { HYPERSWITCH_API_KEY: 'k' }),
    ).join('\n');
    expect(bad).toMatch(/decision_ttl_minutes/);
    expect(bad).toMatch(/decision_max_entries/);
  });
});
