import { FullConfig } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { GlobalBrowserPoolManager } from './utils/BrowserPool';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Global Teardown - Runs once after all tests
 *
 * Responsibilities:
 * 1. Destroy browser pool (if enabled)
 * 2. Archive connector-specific state files for debugging
 * 3. Clean up test artifacts (optional)
 * 4. Log test summary
 */
async function globalTeardown(config: FullConfig) {
  console.log('\n========================================');
  console.log('Playwright Global Teardown');
  console.log('========================================');

  // Destroy browser pool if it was initialized
  const poolManager = (global as any).browserPoolManager as GlobalBrowserPoolManager | null;

  if (poolManager) {
    console.log('\n🧹 Destroying browser pool...');

    // Get final metrics before destroying
    const metrics = poolManager.getAllMetrics();

    // Print final pool metrics
    console.log('\n📊 Final Browser Pool Metrics:');
    metrics.forEach(m => {
      console.log(`\n  ${m.connector.toUpperCase()}:`);
      console.log(`    Total contexts: ${m.totalContexts}`);
      console.log(`    Total allocations: ${m.totalAllocations}`);
      console.log(`    Average reuse: ${m.averageContextReuseCount.toFixed(1)}x`);
      console.log(`    Peak concurrent: ${m.peakConcurrentContexts}`);
      console.log(`    Context creation time: ${m.contextCreationTime}ms`);
    });

    await poolManager.destroyAll();
    console.log('\n✓ Browser pool destroyed');
  }

  // Ensure test-results directory exists
  const resultsDir = path.join(__dirname, 'test-results');
  if (!fs.existsSync(resultsDir)) {
    fs.mkdirSync(resultsDir, { recursive: true });
  }

  // Archive all connector-specific state files
  const stateFiles = fs.readdirSync(__dirname)
    .filter(file => file.match(/^test-state-.+\.json$/));

  if (stateFiles.length > 0) {
    const timestamp = Date.now();

    for (const stateFile of stateFiles) {
      const fullPath = path.join(__dirname, stateFile);
      const archiveFile = path.join(
        resultsDir,
        stateFile.replace('.json', `-${timestamp}.json`)
      );

      // Copy state file to archive
      fs.copyFileSync(fullPath, archiveFile);
      console.log(`✓ Archived: ${stateFile} → ${path.basename(archiveFile)}`);

      // Optionally clean up state file in CI
      if (process.env.CI) {
        fs.unlinkSync(fullPath);
      }
    }

    if (process.env.CI) {
      console.log(`✓ State files cleaned up`);
    }
  } else {
    console.log('⚠️  No connector state files found to archive');
  }

  console.log('✓ Global teardown complete');
  console.log('========================================\n');
}

export default globalTeardown;
