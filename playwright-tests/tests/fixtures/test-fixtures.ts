/**
 * Playwright Test Fixtures
 *
 * Extends Playwright's base test with custom fixtures:
 * - globalState: Shared state management across tests
 * - context: Browser context from the global browser pool
 * - page: Page instance in the pooled context
 *
 * Uses connector-specific state files to support parallel execution:
 * - test-state-stripe.json
 * - test-state-cybersource.json
 *
 * Browser Pool Integration:
 * - Overrides default context/page fixtures to use browser pool
 * - Enables 25 concurrent tests per connector with minimal RAM overhead
 * - Context reuse: ~50 MB per context vs ~1.5 GB per browser
 */

import { test as base, BrowserContext, Page } from '@playwright/test';
import { State, StateData } from '../utils/State';
import { GlobalBrowserPoolManager } from '../../utils/BrowserPool';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Worker-level browser pool manager (lazy initialization)
// Each worker process gets its own pool manager instance
let workerPoolManager: GlobalBrowserPoolManager | null = null;
const poolInitPromise = new Map<string, Promise<void>>();

/**
 * Get or create the browser pool manager for this worker
 * Lazily initializes on first call
 */
async function getPoolManager(): Promise<GlobalBrowserPoolManager> {
  if (workerPoolManager) {
    return workerPoolManager;
  }

  // Check if already initializing
  if (poolInitPromise.has('init')) {
    await poolInitPromise.get('init');
    return workerPoolManager!;
  }

  // Initialize pool
  const initPromise = (async () => {
    const contextsPerBrowser = parseInt(process.env.CONTEXTS_PER_BROWSER || '25');
    const headless = process.env.HEADLESS !== 'false';
    const connectorsList = process.env.PLAYWRIGHT_CONNECTORS
      ? process.env.PLAYWRIGHT_CONNECTORS.split(',')
      : ['stripe', 'cybersource'];

    workerPoolManager = new GlobalBrowserPoolManager({
      contextsPerBrowser,
      launchOptions: { headless },
      verbose: process.env.VERBOSE_POOL === 'true',
    });

    // Initialize pools for configured connectors
    for (const connector of connectorsList) {
      await workerPoolManager.initializePool(connector);
    }
  })();

  poolInitPromise.set('init', initPromise);
  await initPromise;

  return workerPoolManager!;
}

/**
 * Extract connector ID from project name
 * e.g., "stripe-1-core-setup" → "stripe"
 */
function getConnectorFromProject(projectName?: string): string {
  if (!projectName) {
    return process.env.PLAYWRIGHT_CONNECTOR || 'stripe';
  }

  const match = projectName.match(/^(stripe|cybersource)-/);
  if (match) {
    return match[1];
  }

  return process.env.PLAYWRIGHT_CONNECTOR || 'stripe';
}

/**
 * Get the state file path based on the connector being tested
 * Supports parallel execution of multiple connectors
 *
 * Determines connector from:
 * 1. Explicit connectorId parameter
 * 2. Project name (e.g., "stripe-1-core-setup" → "stripe")
 * 3. PLAYWRIGHT_CONNECTOR environment variable
 * 4. Default to 'stripe'
 */
function getStateFilePath(connectorId?: string, projectName?: string): string {
  const connector = connectorId || getConnectorFromProject(projectName);

  // Use connector-specific state file for parallel execution
  return path.join(__dirname, `../../test-state-${connector}.json`);
}

/**
 * Extended test fixtures with browser pool integration
 *
 * Fixtures:
 * - globalState: Shared state management with connector-specific files
 * - context: Browser context from global browser pool (overrides default)
 * - page: Page instance in the pooled context (overrides default)
 *
 * Browser Pool Strategy:
 * - If USE_BROWSER_POOL=true: Use pooled contexts (25 concurrent per connector)
 * - If USE_BROWSER_POOL=false: Use Playwright's default (1 browser per test)
 */
export const test = base.extend<{
  globalState: State;
  context: BrowserContext;
  page: Page;
}>({
  // Global state fixture (unchanged)
  globalState: async ({}, use, testInfo) => {
    // Load state from connector-specific file (created by global-setup)
    let stateData: StateData = {};

    // Get connector-specific state file path based on project name
    const STATE_FILE = getStateFilePath(undefined, testInfo.project.name);

    if (fs.existsSync(STATE_FILE)) {
      try {
        const fileContent = fs.readFileSync(STATE_FILE, 'utf-8');
        stateData = JSON.parse(fileContent);
        // console.log(`✓ Loaded state from ${path.basename(STATE_FILE)} for test: ${testInfo.title}`);
      } catch (error) {
        console.warn(`⚠ Failed to load state file ${STATE_FILE}: ${error}`);
        // Continue with empty state
      }
    } else {
      console.warn(`⚠ State file ${STATE_FILE} not found, using empty state`);
    }

    // Create State instance with loaded data
    const state = new State(stateData);

    // Provide state to test
    await use(state);

    // Save state back to connector-specific file after EVERY test
    // This allows subsequent tests in the same worker to access shared state
    // Safe for parallel execution because each connector uses its own file
    try {
      fs.writeFileSync(STATE_FILE, JSON.stringify(state.data, null, 2));

      const isSetupTest = testInfo.project.name.includes('setup');
      if (isSetupTest) {
        console.log(`✓ State saved to ${path.basename(STATE_FILE)} after ${testInfo.title}`);
        console.log(`  → Saved keys: ${Object.keys(state.data).join(', ')}`);
      }
    } catch (error) {
      console.error(`✗ Failed to save state to ${STATE_FILE}: ${error}`);
      throw error; // Fail the test if state can't be saved
    }
  },

  // Browser context fixture - override to use browser pool
  // When browser pool is disabled, uses Playwright's default context management
  context: async ({ browser }, use, testInfo) => {
    const useBrowserPool = process.env.USE_BROWSER_POOL === 'true';

    // Browser pool mode - use pooled contexts
    if (useBrowserPool) {
      try {
        // Get or create worker-level pool manager
        const poolManager = await getPoolManager();

        // Get connector for this test
        const connector = getConnectorFromProject(testInfo.project.name);

        // Allocate context from pool (this will wait if all contexts are in use)
        const context = await poolManager.allocateContext(connector, testInfo.title);

        // Provide pooled context to test
        await use(context);

        // Release context back to pool for reuse
        await poolManager.releaseContext(connector, context);
      } catch (error) {
        console.error(`⚠️  Browser pool error, falling back to default context:`, error);
        // Fall back to creating context from Playwright's browser
        const context = await browser.newContext();
        await use(context);
        await context.close();
      }
    } else {
      // Traditional mode - create new context per test from Playwright's browser
      const context = await browser.newContext();
      await use(context);
      await context.close();
    }
  },

  // Page fixture - uses page from context (pooled or traditional)
  page: async ({ context }, use) => {
    const page = await context.newPage();
    await use(page);
    await page.close();
  },
});

/**
 * Export expect from Playwright for convenience
 */
export { expect } from '@playwright/test';
