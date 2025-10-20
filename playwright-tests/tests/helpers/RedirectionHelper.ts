/**
 * Redirection Handler for Playwright Tests
 *
 * Ported from Cypress: redirectionHandler.js
 * Handles payment redirections including 3DS authentication
 */

import { Page, FrameLocator } from '@playwright/test';

const TIMEOUT = 90000; // 90 seconds
const WAIT_TIME = 30000; // 30 seconds

/**
 * Handle Stripe 3DS redirection
 * Stripe uses nested iframes for 3DS authentication
 */
export async function handleStripe3DSRedirection(
  page: Page,
  nextActionUrl: string,
  expectedRedirection: string
): Promise<void> {
  console.log(`🔐 Starting Stripe 3DS authentication`);
  console.log(`  → Next action URL: ${nextActionUrl}`);
  console.log(`  → Expected return URL: ${expectedRedirection}`);

  // Navigate to the 3DS authentication page
  await page.goto(nextActionUrl, { waitUntil: 'networkidle', timeout: TIMEOUT });

  try {
    // Wait for the outer iframe to load
    const outerFrame = page.frameLocator('iframe').first();
    console.log(`  → Found outer iframe`);

    // Wait for the inner iframe within the outer iframe
    const innerFrame = outerFrame.frameLocator('iframe').first();
    console.log(`  → Found inner iframe`);

    // Click the "Authorize Test Payment" button
    await innerFrame.locator('#test-source-authorize-3ds').click({ timeout: TIMEOUT });
    console.log(`  → Clicked 3DS authorization button`);

    // Wait for redirection to the return URL
    await page.waitForURL(new RegExp(expectedRedirection), { timeout: TIMEOUT });
    console.log(`✓ Stripe 3DS authentication completed successfully`);
  } catch (error) {
    console.error(`✗ Stripe 3DS authentication failed: ${error}`);
    throw error;
  }
}

/**
 * Handle generic 3DS redirection based on connector
 */
export async function handle3DSRedirection(
  page: Page,
  connectorId: string,
  nextActionUrl: string,
  expectedRedirection: string
): Promise<void> {
  switch (connectorId.toLowerCase()) {
    case 'stripe':
      await handleStripe3DSRedirection(page, nextActionUrl, expectedRedirection);
      break;

    case 'cybersource':
      // Cybersource doesn't require manual 3DS interaction in test mode
      console.log('  → Cybersource 3DS - waiting for auto-redirect');
      await page.goto(nextActionUrl, { waitUntil: 'networkidle', timeout: TIMEOUT });
      await page.waitForURL(new RegExp(expectedRedirection), { timeout: TIMEOUT });
      break;

    default:
      console.log(`  → Unknown connector ${connectorId} - skipping 3DS handling`);
      // Just verify the URLs exist
      if (!nextActionUrl || !expectedRedirection) {
        throw new Error('Missing redirection URLs');
      }
  }
}
