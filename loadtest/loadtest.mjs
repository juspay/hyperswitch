#!/usr/bin/env node
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { fixtures, load } from './k6/scenario-mix/run.mjs';

const root = path.dirname(fileURLToPath(import.meta.url));
const [command, mode = 'single', ...args] = process.argv.slice(2);
async function main() {
  if (!['fixtures', 'load'].includes(command) || !['single', 'tenancy'].includes(mode)) {
    console.log('Usage: node loadtest/loadtest.mjs fixtures|load single|tenancy [--config file]');
    process.exitCode = command === '--help' ? 0 : 1;
    return;
  }
  if (args.length && (args.length !== 2 || args[0] !== '--config')) throw new Error('Expected --config file');
  const configFile = path.resolve(args[1] || path.join(root, 'loadtest.config.json'));
  const config = JSON.parse(await readFile(configFile, 'utf8'));
  const output = path.resolve(path.dirname(configFile), config.output || 'artifacts');
  const fixturePath = path.join(output, mode, 'fixtures');
  await mkdir(fixturePath, { recursive: true, mode: 0o700 });
  if (command === 'fixtures') {
    const provision = { ...config.fixtures, merchant_count: mode === 'single' ? 1 : 1500,
      merchant_id_prefix: `${config.fixtures.merchant_id_prefix || 'loadtest'}_${mode}` };
    const file = path.join(fixturePath, 'provision.json');
    await writeFile(file, JSON.stringify(provision, null, 2), { mode: 0o600 });
    console.log(JSON.stringify(await fixtures(file, fixturePath), null, 2));
  } else {
    const manifest = JSON.parse(await readFile(path.join(fixturePath, 'manifest.json'), 'utf8'));
    const requested = config.fixtures.superposition;
    if (requested && (!isDeepStrictEqual(requested.defaults, manifest.superposition?.defaults)
      || requested.organization_id !== manifest.superposition?.organization_id
      || requested.workspace_id !== manifest.superposition?.workspace_id)) {
      throw new Error(`Superposition configuration differs from ready fixtures; rerun fixtures ${mode}`);
    }
    const { fixtures: _fixtures, output: _output, ...loadConfig } = config;
    const runPath = path.join(output, mode, 'runs', `${new Date().toISOString().replace(/[:.]/g, '-')}-${process.pid}`);
    await mkdir(runPath, { recursive: true, mode: 0o700 });
    const file = path.join(runPath, 'config.json');
    await writeFile(file, JSON.stringify(loadConfig, null, 2), { mode: 0o600 });
    const result = await load(file, fixturePath, runPath);
    console.log(JSON.stringify(result, null, 2));
    process.exitCode = result.exitCode;
  }
}
main().catch(error => { console.error(error.message); process.exitCode = 1; });
