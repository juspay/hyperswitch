import { FullConfig } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Global Teardown - Runs once after all tests
 *
 * Responsibilities:
 * 1. Clean up test artifacts (optional)
 * 2. Log test summary
 * 3. Archive test state for debugging
 */
async function globalTeardown(config: FullConfig) {
  console.log('\n========================================');
  console.log('Playwright Global Teardown');
  console.log('========================================');

  const stateFile = path.join(__dirname, 'test-state.json');

  // Archive state file for debugging
  if (fs.existsSync(stateFile)) {
    const archiveFile = path.join(
      __dirname,
      'test-results',
      `test-state-${Date.now()}.json`
    );

    // Ensure test-results directory exists
    const resultsDir = path.join(__dirname, 'test-results');
    if (!fs.existsSync(resultsDir)) {
      fs.mkdirSync(resultsDir, { recursive: true });
    }

    // Copy state file to archive
    fs.copyFileSync(stateFile, archiveFile);
    console.log(`✓ Test state archived: ${archiveFile}`);

    // Optionally clean up state file (keep it for local debugging)
    if (process.env.CI) {
      // In CI, we can remove it
      fs.unlinkSync(stateFile);
      console.log(`✓ Test state file cleaned up`);
    }
  }

  console.log('✓ Global teardown complete');
  console.log('========================================\n');
}

export default globalTeardown;
