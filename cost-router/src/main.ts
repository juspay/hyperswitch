import 'reflect-metadata';
import { NestFactory } from '@nestjs/core';
import { loadAppConfig } from './app.config';
import { AppModule } from './app.module';

// Ctrl+C / kill: exit normally so 'exit' handlers run (the state-file lock is released there).
for (const signal of ['SIGINT', 'SIGTERM'] as const) process.once(signal, () => process.exit(0));

async function main(): Promise<void> {
  const path = process.env['COST_ROUTER_CONFIG'] ?? 'config.json';
  const config = loadAppConfig(path); // env overrides the file; invalid config fails here, at startup, with one ConfigError
  const app = await NestFactory.create(AppModule.forRoot({ config, defaultMetrics: true }));
  await app.listen(config.port);
}

main().catch((e: unknown) => {
  console.error(e instanceof Error ? e.message : e);
  process.exit(1);
});
