import { FullConfig } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { fileURLToPath } from 'url';
import dotenv from 'dotenv';
import { GlobalBrowserPoolManager } from './utils/BrowserPool';

// Load environment variables from .env file
dotenv.config();

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Global Setup - Runs once before all tests
 *
 * Responsibilities:
 * 1. Initialize connector-specific state files for parallel execution
 * 2. Log system information for debugging
 * 3. Validate required environment variables
 * 4. Initialize browser pool for multi-tab parallel execution (if USE_BROWSER_POOL=true)
 */
async function globalSetup(config: FullConfig) {
  // Log system information
  console.log('\n========================================');
  console.log('Playwright Global Setup');
  console.log('========================================');
  console.log(`Platform: ${os.platform()}`);
  console.log(`Architecture: ${os.arch()}`);
  console.log(`CPUs: ${os.cpus().length}`);
  console.log(`Total RAM: ${(os.totalmem() / 1024 / 1024 / 1024).toFixed(2)} GB`);
  console.log(`Free RAM: ${(os.freemem() / 1024 / 1024 / 1024).toFixed(2)} GB`);
  console.log(`Node Version: ${process.version}`);
  console.log(`Workers: ${config.workers}`);
  console.log('========================================\n');

  // Connectors to test (from environment or default to both)
  const connectorsList = process.env.PLAYWRIGHT_CONNECTORS
    ? process.env.PLAYWRIGHT_CONNECTORS.split(',')
    : ['stripe', 'cybersource'];

  console.log(`Initializing state for connectors: ${connectorsList.join(', ')}\n`);

  // Create state file for each connector
  for (const connector of connectorsList) {
    const stateFile = path.join(__dirname, `test-state-${connector}.json`);

    const initialState = {
      baseUrl: process.env.PLAYWRIGHT_BASEURL || 'http://localhost:8080',
      connectorId: connector,
      adminApiKey: process.env.PLAYWRIGHT_ADMINAPIKEY,
      connectorAuthFilePath: process.env.PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH,
      timestamp: new Date().toISOString(),
      environment: process.env.CI ? 'CI' : 'Local',
    };

    // Validate required environment variables
    if (!initialState.adminApiKey) {
      console.warn(`⚠️  Warning: PLAYWRIGHT_ADMINAPIKEY is not set for ${connector}`);
    }

    if (!initialState.connectorAuthFilePath) {
      console.warn(`⚠️  Warning: PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH is not set for ${connector}`);
    }

    // Write connector-specific state file
    fs.writeFileSync(stateFile, JSON.stringify(initialState, null, 2));
    console.log(`✓ [${connector}] State initialized: ${path.basename(stateFile)}`);
    console.log(`  → Base URL: ${initialState.baseUrl}`);
  }

  console.log('\n✓ All connector states initialized\n');

  // Initialize browser pool for multi-tab parallel execution
  const useBrowserPool = process.env.USE_BROWSER_POOL === 'true';

  if (useBrowserPool) {
    console.log('========================================');
    console.log('Initializing Browser Pool');
    console.log('========================================\n');

    const contextsPerBrowser = parseInt(process.env.CONTEXTS_PER_BROWSER || '25');
    const headless = process.env.HEADLESS !== 'false';

    console.log(`Configuration:`);
    console.log(`  Connectors: ${connectorsList.join(', ')}`);
    console.log(`  Contexts per browser: ${contextsPerBrowser}`);
    console.log(`  Total concurrent tests: ${connectorsList.length * contextsPerBrowser}`);
    console.log(`  Headless: ${headless}\n`);

    // Create global browser pool manager
    const poolManager = new GlobalBrowserPoolManager({
      contextsPerBrowser,
      launchOptions: { headless },
      verbose: process.env.VERBOSE_POOL === 'true',
    });

    // Initialize pools for each connector
    for (const connector of connectorsList) {
      await poolManager.initializePool(connector);
    }

    // Store in global for access by test fixtures
    (global as any).browserPoolManager = poolManager;

    // Log expected RAM usage
    const browserRAM = 0.6; // GB per browser instance
    const contextRAM = 0.05; // GB per context
    const totalRAM = connectorsList.length * (browserRAM + contextsPerBrowser * contextRAM);

    console.log('\n📊 Expected Resource Usage:');
    console.log(`  RAM per browser: ~${browserRAM.toFixed(1)} GB`);
    console.log(`  RAM per context: ~${(contextRAM * 1024).toFixed(0)} MB`);
    console.log(`  Total expected RAM: ~${totalRAM.toFixed(1)} GB`);
    console.log(`  RAM efficiency: ${((contextsPerBrowser * browserRAM) / totalRAM).toFixed(1)}x vs individual browsers\n`);

    console.log('✓ Browser pool initialized\n');
    console.log('========================================\n');
  } else {
    console.log('ℹ️  Browser pool disabled (USE_BROWSER_POOL not set to "true")\n');
    console.log('   Using default Playwright browser management\n');
  }
}

export default globalSetup;
