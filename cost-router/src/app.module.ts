import { Logger, Module, type DynamicModule } from '@nestjs/common';
import type { AppConfig } from './app.config';
import { HttpHyperswitchClient, type HyperswitchClient } from './hyperswitch/client';
import { PaymentsController } from './payments/payments.controller';
import { PaymentsService } from './payments/payments.service';
import { InMemoryDecisionStore } from './payments/decision.store';
import { AttemptDedupe } from './stats/attempt-dedupe';
import { InMemoryAttemptStatsStore, type Clock } from './stats/attempt-stats.store';
import { JsonStateFile } from './storage/json-state-file';
import { RoutingMetrics } from './metrics/routing-metrics';

export interface AppOptions {
  config: AppConfig;
  client?: HyperswitchClient;
  clock?: Clock;
  /** Include Node process metrics on /metrics (main.ts turns this on; tests leave it off). */
  defaultMetrics?: boolean;
}

@Module({})
export class AppModule {
  static forRoot(options: AppOptions): DynamicModule {
    const { config } = options;
    const clock: Clock = options.clock ?? (() => Date.now());
    const client = options.client ?? new HttpHyperswitchClient(config.hyperswitch);
    const e = config.engine;
    const stats = new InMemoryAttemptStatsStore(e.windowMinutes, clock);
    const dedupe = new AttemptDedupe(e.attemptDedupeTtlMinutes, e.attemptDedupeMaxEntries, clock);
    const decisions = new InMemoryDecisionStore(e.decisionTtlMinutes, e.decisionMaxEntries, clock);

    const metrics = new RoutingMetrics({ defaultMetrics: options.defaultMetrics ?? false });
    let persist: (() => void) | undefined;
    if (config.stateFile) {
      // Stats, dedupe and decisions are restored and saved together (see json-state-file.ts).
      const log = new Logger('cost-router');
      const file = new JsonStateFile(config.stateFile);
      const warning = file.acquireLock(); // throws if another live process uses this file
      if (warning) log.warn(warning);
      const saved = file.load(); // throws on an invalid file; null on first start
      if (saved) {
        stats.importState(saved.stats);
        dedupe.importState(saved.dedupe);
        decisions.importState(saved.decisions);
      }
      log.log(`state file ${config.stateFile}: ${saved ? `loaded ${saved.decisions.length} decisions, ${saved.stats.length} stat buckets` : 'new, starting empty'}`);
      persist = () =>
        file.save({ version: 1, savedAtMs: clock(), stats: stats.exportState(), dedupe: dedupe.exportState(), decisions: decisions.exportState() });
    }

    return {
      module: AppModule,
      controllers: [PaymentsController],
      providers: [
        { provide: RoutingMetrics, useValue: metrics },
        {
          provide: PaymentsService,
          useValue: new PaymentsService(config, client, stats, decisions, clock, { dedupe, metrics, ...(persist && { persist }) }),
        },
      ],
    };
  }
}
