import { existsSync, mkdtempSync, readdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Logger } from '@nestjs/common';
import { Test } from '@nestjs/testing';
import { parseAppConfig } from '../src/app.config';
import { AppModule } from '../src/app.module';
import type { HyperswitchClient } from '../src/hyperswitch/client';
import { InMemoryDecisionStore } from '../src/payments/decision.store';
import { PaymentsService } from '../src/payments/payments.service';
import { InMemoryAttemptStatsStore } from '../src/stats/attempt-stats.store';
import { JsonStateFile, StateFileError, type PersistedState } from '../src/storage/json-state-file';
import { counts, rawConfig } from './helpers';

const freshDir = () => mkdtempSync(join(tmpdir(), 'cost-router-state-'));
const emptyState = (): PersistedState => ({ version: 1, savedAtMs: 0, stats: [], dedupe: [], decisions: [] });

describe('JsonStateFile', () => {
  it('returns null when the file does not exist yet, and round-trips a saved state', () => {
    const f = new JsonStateFile(join(freshDir(), 'nested', 'state.json'));
    f.acquireLock(); // also creates the parent directory
    expect(f.load()).toBeNull();
    const state: PersistedState = {
      ...emptyState(),
      savedAtMs: 123,
      stats: [{ connector: 'A', currency: 'INR', minute: 5, counts: counts({ success: 2, technical_failure: 1 }) }],
      dedupe: [{ key: '["p1","a1","terminal"]', expiresAt: 999 }],
    };
    f.save(state);
    expect(f.load()).toEqual(state);
    f.releaseLock();
  });

  it('writes through a temp file and leaves only the target and nothing half-written', () => {
    const dir = freshDir();
    const f = new JsonStateFile(join(dir, 'state.json'));
    f.save(emptyState());
    f.save({ ...emptyState(), savedAtMs: 2 });
    expect(readdirSync(dir)).toEqual(['state.json']);
    expect(JSON.parse(readFileSync(join(dir, 'state.json'), 'utf8')).savedAtMs).toBe(2); // human-readable JSON
  });

  it('refuses an invalid or wrongly shaped file with a clear error that does not echo its content', () => {
    const dir = freshDir();
    const path = join(dir, 'state.json');
    writeFileSync(path, '{"secret": "do-not-echo", ');
    expect(() => new JsonStateFile(path).load()).toThrow(StateFileError);
    expect(() => new JsonStateFile(path).load()).toThrow(/not valid JSON/);
    expect(() => new JsonStateFile(path).load()).not.toThrow(/do-not-echo/);
    writeFileSync(path, JSON.stringify({ ...emptyState(), version: 2 }));
    expect(() => new JsonStateFile(path).load()).toThrow(/unexpected shape/);
  });

  it('allows only one live process per file, and takes over a stale lock', () => {
    const path = join(freshDir(), 'state.json');
    const first = new JsonStateFile(path);
    expect(first.acquireLock()).toBeNull();
    expect(() => new JsonStateFile(path).acquireLock()).toThrow(new RegExp(`in use by process ${process.pid}`));
    first.releaseLock();
    expect(existsSync(`${path}.lock`)).toBe(false);

    writeFileSync(`${path}.lock`, '999999'); // a pid that is not running (crashed owner)
    const second = new JsonStateFile(path);
    expect(second.acquireLock()).toMatch(/stale lock/);
    expect(readFileSync(`${path}.lock`, 'utf8')).toBe(String(process.pid));
    second.releaseLock();
  });
});

describe('restart with a state file', () => {
  const body = { amount: 100000, currency: 'INR', confirm: true };
  let calls = 0;
  const client: HyperswitchClient = {
    createPayment: async () => {
      calls += 1;
      return { status: 200, body: { status: 'succeeded', connector: 'B', attempts: [{ attempt_id: `att_${calls}`, connector: 'B', status: 'charged' }] } };
    },
    retrieveAttempts: async () => [],
  };
  const boot = async (statePath: string) => {
    const config = parseAppConfig(
      { port: 0, hyperswitch: { base_url: 'http://unused.invalid', api_key: 'k' }, candidates: [{ connector: 'A', merchant_connector_id: 'mca_A' }, { connector: 'B', merchant_connector_id: 'mca_B' }], state_file: statePath, routing: rawConfig({ min_sample_size: 1 }) },
      {},
    );
    const mod = await Test.createTestingModule({ imports: [AppModule.forRoot({ config, client, clock: () => 0 })] }).compile();
    return mod.get(PaymentsService);
  };
  let log: jest.SpyInstance;
  beforeEach(() => {
    calls = 0;
    log = jest.spyOn(Logger.prototype, 'log').mockImplementation(() => undefined);
  });
  afterEach(() => log.mockRestore());

  it('keeps statistics, de-duplication and idempotent replay across a restart', async () => {
    const path = join(freshDir(), 'state.json');
    const before = await boot(path);
    await before.create('p1', body, {});
    expect(calls).toBe(1);
    unlinkSync(`${path}.lock`); // simulate the first process exiting (its exit handler removes the lock)

    const after = await boot(path);
    // Replay: answered from the restored decision store, Hyperswitch is not called again.
    const replay = await after.create('p1', body, {});
    expect(replay.replayed).toBe(true);
    expect(calls).toBe(1);
    // Changed body for the same id is still a conflict after the restart.
    await expect(after.create('p1', { ...body, amount: 1 }, {})).rejects.toMatchObject({ status: 409 });
    // De-duplication survived: re-reading p1's attempt does not count it twice.
    after.ingestAttempts('p1', 'INR', [{ attemptId: 'att_1', connector: 'B', status: 'charged', errorCode: null }]);
    // Statistics survived: the next decision sees B's one success from before the restart.
    const p2 = await after.create('p2', body, {});
    const observedB = p2.record.decision!.trace.candidates.find((c) => c.connector === 'B')!.observed;
    expect(observedB.success).toBe(1);
    const saved = JSON.parse(readFileSync(path, 'utf8')) as PersistedState;
    expect(saved.decisions.map((d) => d.record.paymentId)).toEqual(['p1', 'p2']);
    unlinkSync(`${path}.lock`);
  });

  it('a failed state-file write is logged and never fails the payment', async () => {
    const error = jest.spyOn(Logger.prototype, 'error').mockImplementation(() => undefined);
    const config = parseAppConfig({ port: 0, hyperswitch: { base_url: 'http://unused.invalid', api_key: 'k' }, candidates: [{ connector: 'B', merchant_connector_id: 'mca_B' }], routing: rawConfig() }, {});
    const service = new PaymentsService(config, client, new InMemoryAttemptStatsStore(60, () => 0), new InMemoryDecisionStore(60, 100, () => 0), () => 0, {
      persist: () => {
        throw new Error('disk full');
      },
    });
    const out = await service.create('p9', body, {});
    expect(out.record.response?.status).toBe(200);
    expect(error).toHaveBeenCalledWith(expect.stringMatching(/state file write failed.*disk full/));
    error.mockRestore();
  });
});
