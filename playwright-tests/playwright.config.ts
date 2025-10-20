import { defineConfig, devices } from '@playwright/test';
import * as os from 'os';

/**
 * Get optimal worker count based on environment and available RAM
 */
function getOptimalWorkers(): number {
  if (process.env.CI) {
    const totalRAM = os.totalmem() / 1024 / 1024 / 1024; // GB
    console.log(`CI Environment - Total RAM: ${totalRAM.toFixed(2)} GB`);

    if (totalRAM < 8) return 1;   // Low RAM: sequential
    if (totalRAM < 16) return 2;  // GitHub runners: 2 connectors in parallel
    return 2;  // Keep it 2 for stability (Stripe + Cybersource)
  }

  // Local development: 2 connectors in parallel
  return 2;
}

/**
 * Playwright Configuration for Hyperswitch E2E Tests
 *
 * Architecture:
 * 1. Setup tests (0000-0003): Run sequentially to create shared state
 * 2. Parallel tests (0004+): Run in 20 isolated browser contexts (tabs) per connector
 *
 * Execution:
 * - Stripe: 20 tabs running simultaneously
 * - Cybersource: 20 tabs running simultaneously
 * - Both connectors run in parallel
 *
 * RAM Usage: ~11.6 GB (5.8 GB per connector)
 */
export default defineConfig({
  testDir: './tests/e2e',

  // Global setup/teardown
  globalSetup: './global-setup.ts',
  globalTeardown: './global-teardown.ts',

  // Timeouts
  timeout: 120000, // 2 minutes per test
  expect: {
    timeout: 30000, // 30 seconds for assertions
  },

  // Workers: 1 (sequential execution to share state properly)
  workers: 1,

  // Test execution
  fullyParallel: false, // Controlled by project dependencies
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,

  // Reporter configuration
  reporter: process.env.CI
    ? [
        ['html', { outputFolder: 'playwright-report', open: 'never' }],
        ['json', { outputFile: 'test-results/results.json' }],
        ['list'],
        ['github'],
      ]
    : [
        ['html', { outputFolder: 'playwright-report', open: 'never' }],
        ['json', { outputFile: 'test-results/results.json' }],
        ['list'],
      ],

  // Shared test configuration
  use: {
    // Base URL from environment
    baseURL: process.env.PLAYWRIGHT_BASEURL || 'http://localhost:8080',

    // Headless mode (can be overridden with HEADLESS=false)
    headless: process.env.HEADLESS !== 'false',

    // Tracing and debugging
    trace: process.env.CI ? 'retain-on-failure' : 'on-first-retry',
    screenshot: 'only-on-failure',
    video: process.env.CI ? 'retain-on-failure' : 'off',

    // Viewport
    viewport: { width: 1280, height: 720 },

    // Slow down in headed mode for visibility
    ...(process.env.HEADLESS === 'false' && { slowMo: 100 }),

    // Action timeout
    actionTimeout: 30000,
    navigationTimeout: 60000,
  },

  /**
   * Project Configuration
   *
   * Sequential Setup Projects (0000-0003):
   * - Create merchant, API keys, customer, connectors
   * - Save state to test-state.json
   *
   * Parallel Test Projects (0004-0032):
   * - Each test file runs in its own browser context (tab)
   * - 29 tests run simultaneously per connector
   * - Both connectors (Stripe + Cybersource) run in parallel
   */
  projects: [
    // ==========================================
    //  SETUP TESTS (Sequential Dependencies)
    // ==========================================
    {
      name: '1-core-setup',
      testMatch: '**/setup/00000-CoreFlows.spec.ts',
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: '2-account-setup',
      testMatch: '**/setup/00001-AccountCreate.spec.ts',
      dependencies: ['1-core-setup'],
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: '3-customer-setup',
      testMatch: '**/setup/00002-CustomerCreate.spec.ts',
      dependencies: ['2-account-setup'],
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: '4-connector-setup',
      testMatch: '**/setup/00003-ConnectorCreate.spec.ts',
      dependencies: ['3-customer-setup'],
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
    },

    // ==========================================
    //  PARALLEL TESTS (Multi-Tab Execution)
    //  Tests 00004-00032 (29 tests total)
    //  Each test file = 1 browser context (tab)
    // ==========================================
    {
      name: 'parallel-tests',
      testMatch: [
        '**/spec/0000[4-9]-*.spec.ts',  // 00004-00009
        '**/spec/0001[0-9]-*.spec.ts',  // 00010-00019
        '**/spec/0002[0-9]-*.spec.ts',  // 00020-00029
        '**/spec/0003[0-2]-*.spec.ts',  // 00030-00032
      ],
      testIgnore: '**/setup/**', // Ignore setup directory
      dependencies: ['4-connector-setup'],
      fullyParallel: false, // Tests within each file run sequentially (they share state)
      use: {
        ...devices['Desktop Chrome'],
        // Each test file gets its own browser context (tab)
      },
    },
  ],

  // Output folders
  outputDir: 'test-results',
});
