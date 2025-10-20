/**
 * Payment Common Configurations
 *
 * Shared payment configuration data across all connector types
 * Ported from Cypress configs/Payment/Commons.js
 */

export const defaultPaymentConfig = {
  currency: 'USD',
  amount: 6000,
  authentication_type: 'no_three_ds',
  capture_method: 'automatic',
};

export const threeDSPaymentConfig = {
  ...defaultPaymentConfig,
  authentication_type: 'three_ds',
};

export const manualCaptureConfig = {
  ...defaultPaymentConfig,
  capture_method: 'manual',
};

// Browser info for 3DS authentication
export const browserInfo = {
  user_agent:
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  accept_header:
    'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8',
  language: 'en-US',
  color_depth: 32,
  screen_height: 1117,
  screen_width: 1728,
  time_zone: -330,
  java_enabled: true,
  java_script_enabled: true,
  ip_address: '127.0.0.1',
};

// Billing address
export const billingAddress = {
  address: {
    line1: '1467',
    line2: 'Harrison Street',
    line3: 'Harrison Street',
    city: 'San Francisco',
    state: 'California',
    zip: '94122',
    country: 'US',
    first_name: 'john',
    last_name: 'doe',
  },
};

// Default metadata
export const defaultMetadata = {
  udf1: 'value1',
  new_customer: 'true',
  login_date: '2019-09-10T10:11:12Z',
};

// Payment methods enabled configuration
// Re-exported from Commons for convenience
import { payment_methods_enabled as commonPaymentMethods } from '../Commons';
export { payment_methods_enabled } from '../Commons';
