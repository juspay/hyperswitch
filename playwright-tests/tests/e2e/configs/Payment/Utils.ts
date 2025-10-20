/**
 * Payment Utility Functions
 *
 * Helper functions for payment-related operations
 * Ported from Cypress configs/Payment/Utils.js
 */

// Re-export getConnectorDetails from Commons
export { getConnectorDetails } from '../Commons';

/**
 * Connector lists for testing different connector combinations
 */
export const CONNECTOR_LISTS = {
  stripe: ['stripe', 'stripe_test'],
  cybersource: ['cybersource', 'cybersource_test'],
  all: ['stripe', 'cybersource', 'adyen', 'checkout'],
  INCLUDE: [] as string[], // Used to filter connectors in tests
  MANDATES_USING_NTID_PROXY: [] as string[],
  INCREMENTAL_AUTH: [] as string[],
  DDC_RACE_CONDITION: [] as string[],
  OVERCAPTURE: [] as string[],
  MANUAL_RETRY: [] as string[],
};

/**
 * Get value from nested object using dot notation path
 * Example: getValueByKey(obj, 'payment.amount') -> obj.payment.amount
 */
export function getValueByKey(obj: any, path: string): any {
  return path.split('.').reduce((current, key) => current?.[key], obj);
}

/**
 * Set value in nested object using dot notation path
 * Example: setValueByKey(obj, 'payment.amount', 1000)
 */
export function setValueByKey(obj: any, path: string, value: any): void {
  const keys = path.split('.');
  const lastKey = keys.pop();
  const target = keys.reduce((current, key) => {
    if (!current[key]) current[key] = {};
    return current[key];
  }, obj);
  if (lastKey) target[lastKey] = value;
}

/**
 * Should continue with the test based on response data
 * Used to check if test should proceed or skip
 */
export function shouldContinue(data: any): boolean {
  // Implementation depends on Cypress logic
  // Placeholder for now
  return data?.should_continue !== false;
}

/**
 * Alias for shouldContinue (used in some tests)
 */
export function shouldContinueFurther(data: any): boolean {
  return shouldContinue(data);
}

/**
 * Check if connector should be included in test run
 * Used for connector-specific test filtering
 */
export function shouldIncludeConnector(
  connectorId: string,
  includeList?: string[]
): boolean {
  if (!includeList || includeList.length === 0) return true;
  return includeList.includes(connectorId.toLowerCase());
}

/**
 * Convert connector response to standard format
 */
export function normalizeConnectorResponse(response: any, connectorId: string): any {
  // Connector-specific response normalization
  // Placeholder implementation
  return response;
}

/**
 * Validate payment response structure
 */
export function validatePaymentResponse(response: any): boolean {
  return (
    response &&
    typeof response === 'object' &&
    'payment_id' in response &&
    'status' in response
  );
}

/**
 * Extract error details from response
 */
export function extractErrorDetails(response: any): {
  code?: string;
  message?: string;
  type?: string;
} {
  return {
    code: response?.error?.code,
    message: response?.error?.message,
    type: response?.error?.type,
  };
}

/**
 * Check if payment method is supported by connector
 */
export function isPaymentMethodSupported(
  connectorId: string,
  paymentMethod: string,
  paymentMethodType?: string
): boolean {
  // This would check against connector capabilities
  // Placeholder implementation
  return true;
}

/**
 * Generate unique order ID for testing
 */
export function generateOrderId(): string {
  return `order_${Date.now()}_${Math.random().toString(36).substring(7)}`;
}

/**
 * Parse 3DS redirect URL for authentication
 */
export function parse3DSRedirectUrl(url: string): {
  authUrl?: string;
  params?: Record<string, string>;
} {
  try {
    const urlObj = new URL(url);
    const params: Record<string, string> = {};
    urlObj.searchParams.forEach((value, key) => {
      params[key] = value;
    });
    return { authUrl: urlObj.href, params };
  } catch {
    return {};
  }
}
