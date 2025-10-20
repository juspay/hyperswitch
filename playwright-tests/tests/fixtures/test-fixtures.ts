/**
 * Playwright Test Fixtures
 *
 * Extends Playwright's base test with custom fixtures:
 * - globalState: Shared state management across tests
 */

import { test as base } from '@playwright/test';
import { State, StateData } from '../utils/State';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const STATE_FILE = path.join(__dirname, '../../test-state.json');

/**
 * Extended test fixtures with globalState
 *
 * globalState is test-scoped, with state persisted to file after EVERY test.
 * This allows tests within a describe.serial block to share state properly.
 */
export const test = base.extend<{ globalState: State }>({
  globalState: async ({}, use, testInfo) => {
    // Load state from file (created by global-setup)
    let stateData: StateData = {};

    if (fs.existsSync(STATE_FILE)) {
      try {
        const fileContent = fs.readFileSync(STATE_FILE, 'utf-8');
        stateData = JSON.parse(fileContent);
        // console.log(`✓ Loaded state for test: ${testInfo.title}`);
      } catch (error) {
        console.warn(`⚠ Failed to load state file: ${error}`);
        // Continue with empty state
      }
    } else {
      console.warn(`⚠ State file not found, using empty state`);
    }

    // Create State instance with loaded data
    const state = new State(stateData);

    // Provide state to test
    await use(state);

    // Save state back to file after EVERY test
    // This allows subsequent tests in the same describe.serial block to access shared state
    // Safe because tests run sequentially (workers: 1)
    try {
      fs.writeFileSync(STATE_FILE, JSON.stringify(state.data, null, 2));

      const isSetupTest = testInfo.project.name.includes('setup');
      if (isSetupTest) {
        console.log(`✓ State saved after ${testInfo.title}`);
        console.log(`  → Saved keys: ${Object.keys(state.data).join(', ')}`);
      }
    } catch (error) {
      console.error(`✗ Failed to save state: ${error}`);
      throw error; // Fail the test if state can't be saved
    }
  },
});

/**
 * Export expect from Playwright for convenience
 */
export { expect } from '@playwright/test';
