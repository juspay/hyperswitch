import { readFileSync } from 'node:fs';
import { z } from 'zod';
import { ConfigError, parseEngineConfig, type EngineConfig } from './engine/config';
import type { Candidate } from './engine/types';

export interface AppConfig {
  engine: EngineConfig;
  candidates: Candidate[];
  hyperswitch: { baseUrl: string; apiKey: string };
  port: number;
  /** Optional JSON state file (stats, dedupe, decisions). null = in memory only. */
  stateFile: string | null;
}

export type Env = Readonly<Record<string, string | undefined>>;

/**
 * Environment overrides the JSON file: HYPERSWITCH_API_KEY -> hyperswitch.api_key,
 * HYPERSWITCH_BASE_URL -> hyperswitch.base_url, PORT -> port, COST_ROUTER_STATE_FILE -> state_file.
 * (COST_ROUTER_CONFIG, the file path, is read by main.ts.) The key is never echoed in any message.
 */
const appSchema = z
  .object({
    port: z.number().int().min(0).max(65535).optional(),
    hyperswitch: z
      .object({ base_url: z.string().optional(), api_key: z.string().optional() })
      .strict()
      .default({}),
    candidates: z
      .array(z.object({ connector: z.string().min(1), merchant_connector_id: z.string().min(1) }).strict())
      .min(1),
    state_file: z.string().min(1).optional(),
    routing: z.unknown(), // engine config, validated by parseEngineConfig
  })
  .strict();

function validUrl(value: string): boolean {
  try {
    new URL(value);
    return true;
  } catch {
    return false;
  }
}

/** Validates file + env together and throws ONE ConfigError listing every problem. */
export function parseAppConfig(raw: unknown, env: Env = process.env): AppConfig {
  const issues: string[] = [];
  const parsed = appSchema.safeParse(raw);
  if (!parsed.success) {
    for (const i of parsed.error.issues) issues.push(`${i.path.join('.') || '(root)'}: ${i.message}`);
  }
  const file = parsed.success ? parsed.data : undefined;

  const apiKey = env['HYPERSWITCH_API_KEY'] || file?.hyperswitch.api_key;
  if (!apiKey) issues.push('api key missing: set HYPERSWITCH_API_KEY (or hyperswitch.api_key in the config file)');

  const baseUrl = env['HYPERSWITCH_BASE_URL'] || file?.hyperswitch.base_url;
  if (!baseUrl) issues.push('base url missing: set HYPERSWITCH_BASE_URL (or hyperswitch.base_url in the config file)');
  else if (!validUrl(baseUrl)) issues.push('base url invalid: HYPERSWITCH_BASE_URL / hyperswitch.base_url must be an absolute URL');

  let port = file?.port;
  if (env['PORT']) {
    const n = Number(env['PORT']);
    if (/^\d+$/.test(env['PORT']) && n <= 65535) port = n;
    else issues.push('port invalid: PORT must be an integer between 0 and 65535');
  }
  if (port === undefined && !issues.some((i) => i.startsWith('port'))) {
    issues.push('port missing: set PORT (or port in the config file)');
  }

  let engine: EngineConfig | undefined;
  try {
    engine = parseEngineConfig(file ? file.routing : (raw as { routing?: unknown } | null)?.routing);
  } catch (e) {
    if (e instanceof ConfigError) issues.push(...e.issues.map((i) => `routing.${i}`));
    else throw e;
  }

  if (issues.length > 0 || !file || !engine || !apiKey || !baseUrl || port === undefined) {
    throw new ConfigError(issues);
  }
  return {
    engine,
    candidates: file.candidates.map((c) => ({ connector: c.connector, merchantConnectorId: c.merchant_connector_id })),
    hyperswitch: { baseUrl, apiKey },
    port,
    stateFile: env['COST_ROUTER_STATE_FILE'] || file.state_file || null,
  };
}

/** Reads the JSON file with clear errors (no raw ENOENT/SyntaxError; parser messages can echo file content, so they are not forwarded). */
export function readConfigFile(path: string): unknown {
  let text: string;
  try {
    text = readFileSync(path, 'utf8');
  } catch (e) {
    const code = (e as NodeJS.ErrnoException).code;
    throw new ConfigError([
      code === 'ENOENT'
        ? `config file not found: ${path} (set COST_ROUTER_CONFIG to its path)`
        : `config file unreadable: ${path} (${code ?? 'unknown error'})`,
    ]);
  }
  try {
    return JSON.parse(text);
  } catch (e) {
    const pos = /position (\d+)/.exec(e instanceof Error ? e.message : '')?.[1];
    throw new ConfigError([`config file is not valid JSON: ${path}${pos ? ` (near character ${pos})` : ''}`]);
  }
}

export function loadAppConfig(path: string, env: Env = process.env): AppConfig {
  return parseAppConfig(readConfigFile(path), env);
}
