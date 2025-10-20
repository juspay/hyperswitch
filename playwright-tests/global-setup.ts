import { FullConfig } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { fileURLToPath } from 'url';
import dotenv from 'dotenv';

// Load environment variables from .env file
dotenv.config();

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Global Setup - Runs once before all tests
 *
 * Responsibilities:
 * 1. Initialize test-state.json with environment variables
 * 2. Log system information for debugging
 * 3. Validate required environment variables
 */
async function globalSetup(config: FullConfig) {
  const stateFile = path.join(__dirname, 'test-state.json');

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

  // Initialize state file with environment variables
  const initialState = {
    baseUrl: process.env.PLAYWRIGHT_BASEURL || 'http://localhost:8080',
    connectorId: process.env.PLAYWRIGHT_CONNECTOR || 'stripe', // Will be overridden per test
    adminApiKey: process.env.PLAYWRIGHT_ADMINAPIKEY,
    connectorAuthFilePath: process.env.PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH,
    timestamp: new Date().toISOString(),
    environment: process.env.CI ? 'CI' : 'Local',
  };

  // Validate required environment variables
  if (!initialState.adminApiKey) {
    console.warn('⚠️  Warning: PLAYWRIGHT_ADMINAPIKEY is not set');
  }

  if (!initialState.connectorAuthFilePath) {
    console.warn('⚠️  Warning: PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH is not set');
  }

  // Write initial state
  fs.writeFileSync(stateFile, JSON.stringify(initialState, null, 2));
  console.log(`✓ Global state initialized: ${stateFile}`);
  console.log(`✓ Base URL: ${initialState.baseUrl}`);
  console.log(`✓ Connector: ${initialState.connectorId}\n`);
}

export default globalSetup;
