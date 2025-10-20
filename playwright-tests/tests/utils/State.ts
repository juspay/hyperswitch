/**
 * State Management Class for Playwright Tests
 *
 * Manages global state across test execution.
 * Ported from Cypress State.js
 */

export interface StateData {
  [key: string]: any;
  connectorId?: string;
  baseUrl?: string;
  adminApiKey?: string;
  email?: string;
  password?: string;
  connectorAuthFilePath?: string;
  merchantId?: string;
  apiKey?: string;
  publishableKey?: string;
  customerId?: string;
  paymentId?: string;
  clientSecret?: string;
  stripeConnectorId?: string;
  cybersourceConnectorId?: string;
}

export class State {
  data: StateData;

  constructor(initialData: StateData = {}) {
    this.data = initialData;

    // Initialize from environment if not already set
    if (!this.data.connectorId) {
      this.data.connectorId =
        process.env.PLAYWRIGHT_CONNECTOR || 'stripe';
    }

    if (!this.data.baseUrl) {
      this.data.baseUrl =
        process.env.PLAYWRIGHT_BASEURL || 'http://localhost:8080';
    }

    if (!this.data.adminApiKey) {
      this.data.adminApiKey = process.env.PLAYWRIGHT_ADMINAPIKEY;
    }

    if (!this.data.email) {
      this.data.email = process.env.HS_EMAIL;
    }

    if (!this.data.password) {
      this.data.password = process.env.HS_PASSWORD;
    }

    if (!this.data.connectorAuthFilePath) {
      this.data.connectorAuthFilePath =
        process.env.PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH;
    }
  }

  /**
   * Set a value in the state
   */
  set(key: string, value: any): void {
    this.data[key] = value;
  }

  /**
   * Get a value from the state
   */
  get(key: string): any {
    return this.data[key];
  }

  /**
   * Check if a key exists in the state
   */
  has(key: string): boolean {
    return key in this.data;
  }

  /**
   * Delete a key from the state
   */
  delete(key: string): void {
    delete this.data[key];
  }

  /**
   * Clear all state data
   */
  clear(): void {
    this.data = {};
  }

  /**
   * Get all state data as a plain object
   */
  getAll(): StateData {
    return { ...this.data };
  }

  /**
   * Merge new data into existing state
   */
  merge(newData: Partial<StateData>): void {
    this.data = { ...this.data, ...newData };
  }
}

export default State;
