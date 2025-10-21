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
 * - Browser Pool Strategy: Reusable browser contexts for maximum parallelism
 * - 2 browsers (Stripe + Cybersource)
 * - 25 contexts per browser (configurable via CONTEXTS_PER_BROWSER env var)
 * - 50 total concurrent tests (maximum throughput with minimal RAM)
 *
 * Execution Flow:
 * 1. Setup Phase (Sequential): Tests 0000-0003 run sequentially per connector
 * 2. Parallel Phase: Tests 0004+ run with 50 concurrent browser contexts
 *
 * Browser Pool Mode (USE_BROWSER_POOL=true):
 * - Stripe: 1 browser with 25 reusable contexts
 * - Cybersource: 1 browser with 25 reusable contexts
 * - Total: 50 concurrent tests, ~14-15 GB RAM
 * - Speedup: ~20x vs sequential execution
 *
 * Traditional Mode (USE_BROWSER_POOL=false):
 * - Each test gets its own browser instance (Playwright default)
 * - Sequential execution, higher resource usage
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

  // Workers Configuration:
  // - Browser Pool Mode: Can use many workers (25-50) since each uses pooled context
  // - Traditional Mode: Limited to 2 (1 per connector) to control RAM usage
  // The actual parallelism limit is set by CONTEXTS_PER_BROWSER (default: 25)
  workers: process.env.USE_BROWSER_POOL === 'true'
    ? parseInt(process.env.CONTEXTS_PER_BROWSER || '25')
    : 2,

  // Test execution
  fullyParallel: false, // Controlled by project dependencies
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,

  // Reporter configuration
  reporter: process.env.CI
    ? [
        ['./reporters/performance-reporter.ts'],
        ['html', { outputFolder: 'playwright-report', open: 'never' }],
        ['json', { outputFile: 'test-results/results.json' }],
        ['list'],
        ['github'],
      ]
    : [
        ['./reporters/performance-reporter.ts'],
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
   * Project Configuration - Parallel Connector Testing
   *
   * Architecture:
   * - Stripe and Cybersource run in parallel (workers: 2)
   * - Each connector has its own setup → tests pipeline
   * - State isolated via connector-specific files (test-state-stripe.json, test-state-cybersource.json)
   *
   * Execution Flow:
   * ┌─────────────────────────────────────────────┐
   * │ Worker 1: Stripe                            │
   * │ stripe-1-core → stripe-2-account →          │
   * │ stripe-3-customer → stripe-4-connector →    │
   * │ stripe-parallel-tests                       │
   * └─────────────────────────────────────────────┘
   * ┌─────────────────────────────────────────────┐
   * │ Worker 2: Cybersource                       │
   * │ cybersource-1-core → cybersource-2-account →│
   * │ cybersource-3-customer → cybersource-4-connector → │
   * │ cybersource-parallel-tests                  │
   * └─────────────────────────────────────────────┘
   */
  projects: [
    // ==========================================
    //  STRIPE - Setup Tests (Sequential)
    // ==========================================
    {
      name: 'stripe-1-core-setup',
      testMatch: '**/setup/00000-CoreFlows.spec.ts',
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
      metadata: { connector: 'stripe' },
    },
    {
      name: 'stripe-2-account-setup',
      testMatch: '**/setup/00001-AccountCreate.spec.ts',
      dependencies: ['stripe-1-core-setup'],
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: 'stripe-3-customer-setup',
      testMatch: '**/setup/00002-CustomerCreate.spec.ts',
      dependencies: ['stripe-2-account-setup'],
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: 'stripe-4-connector-setup',
      testMatch: '**/setup/00003-ConnectorCreate.spec.ts',
      dependencies: ['stripe-3-customer-setup'],
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: 'stripe-parallel-tests',
      testMatch: [
        '**/spec/0000[4-9]-*.spec.ts',
        '**/spec/0001[0-9]-*.spec.ts',
        '**/spec/0002[0-9]-*.spec.ts',
        '**/spec/0003[0-2]-*.spec.ts',
      ],
      testIgnore: '**/setup/**',
      dependencies: ['stripe-4-connector-setup'],
      // Enable full parallelism for tests 00004+
      // With browser pool: 25 concurrent contexts
      // Without browser pool: Limited by workers config
      fullyParallel: true,
      use: {
        ...devices['Desktop Chrome'],
      },
    },

    // ==========================================
    //  CYBERSOURCE - Setup Tests (Sequential)
    // ==========================================
    {
      name: 'cybersource-1-core-setup',
      testMatch: '**/setup/00000-CoreFlows.spec.ts',
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: 'cybersource-2-account-setup',
      testMatch: '**/setup/00001-AccountCreate.spec.ts',
      dependencies: ['cybersource-1-core-setup'],
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: 'cybersource-3-customer-setup',
      testMatch: '**/setup/00002-CustomerCreate.spec.ts',
      dependencies: ['cybersource-2-account-setup'],
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: 'cybersource-4-connector-setup',
      testMatch: '**/setup/00003-ConnectorCreate.spec.ts',
      dependencies: ['cybersource-3-customer-setup'],
      fullyParallel: false,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: 'cybersource-parallel-tests',
      testMatch: [
        '**/spec/0000[4-9]-*.spec.ts',
        '**/spec/0001[0-9]-*.spec.ts',
        '**/spec/0002[0-9]-*.spec.ts',
        '**/spec/0003[0-2]-*.spec.ts',
      ],
      testIgnore: '**/setup/**',
      dependencies: ['cybersource-4-connector-setup'],
      // Enable full parallelism for tests 00004+
      // With browser pool: 25 concurrent contexts
      // Without browser pool: Limited by workers config
      fullyParallel: true,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
  ],

  // Output folders
  outputDir: 'test-results',
});
