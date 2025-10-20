/**
 * API Helpers for Playwright Tests
 *
 * Ported from Cypress commands.js
 * Provides typed HTTP request helpers for Hyperswitch API testing
 */

import { APIRequestContext, expect } from '@playwright/test';
import { State } from '../utils/State';
import * as RequestBodyUtils from '../utils/RequestBodyUtils';
import * as fs from 'fs';

/**
 * Log request ID for debugging
 */
function logRequestId(xRequestId?: string): void {
  if (xRequestId) {
    console.log(`  → x-request-id: ${xRequestId}`);
  } else {
    console.log(`  ⚠ x-request-id not available in response headers`);
  }
}

/**
 * Validate error message fields
 */
function validateErrorMessage(responseBody: any): void {
  if (responseBody.status !== 'failed') {
    expect(responseBody.error_message, 'error_message').toBeNull();
    expect(responseBody.error_code, 'error_code').toBeNull();
  }
}

/**
 * Get value from connector auth file by key
 */
function getValueByKey(
  jsonString: string,
  key: string,
  profileIndex?: number
): { authDetails: any; stateUpdate?: any } {
  const json = JSON.parse(jsonString);
  let connectorData = json[key];

  if (!connectorData) {
    return { authDetails: null };
  }

  // If multiple profiles exist, use the specified index
  if (profileIndex !== undefined && Array.isArray(connectorData)) {
    const profileData = connectorData[profileIndex];
    return profileData ? { authDetails: profileData } : { authDetails: null };
  }

  // Handle array format
  if (Array.isArray(connectorData)) {
    return { authDetails: connectorData[0] };
  }

  // Handle nested connector structure (e.g., connector_1, connector_2)
  // Check if this is a nested object with connector_X keys
  const keys = Object.keys(connectorData);
  console.log(`  [DEBUG] Keys for ${key}:`, keys);

  if (keys.length > 0 && keys[0].startsWith('connector_')) {
    // Extract the first connector configuration
    const firstConnectorKey = keys[0];
    console.log(`  [DEBUG] Extracting nested connector: ${firstConnectorKey}`);
    const extracted = connectorData[firstConnectorKey];
    console.log(`  [DEBUG] Has connector_account_details:`, !!extracted?.connector_account_details);
    return { authDetails: extracted };
  }

  return { authDetails: connectorData };
}

/**
 * Core API Helper Class
 */
export class ApiHelpers {
  constructor(private request: APIRequestContext, private state: State) {}

  /**
   * Health Check
   */
  async healthCheck(): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const url = `${baseUrl}/health`;

    const response = await this.request.get(url, {
      headers: {
        Accept: 'application/json',
      },
    });

    logRequestId(response.headers()['x-request-id']);

    expect(response.status()).toBe(200);
    const body = await response.text();
    expect(body).toBe('health is good');

    console.log('✓ Health check passed');
  }

  /**
   * Merchant Create
   */
  async merchantCreateCall(merchantCreateBody: any): Promise<void> {
    const randomMerchantId = RequestBodyUtils.generateRandomString();
    RequestBodyUtils.setMerchantId(merchantCreateBody, randomMerchantId);
    this.state.set('merchantId', randomMerchantId);

    const response = await this.request.post(
      `${this.state.get('baseUrl')}/accounts`,
      {
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
          'api-key': this.state.get('adminApiKey'),
        },
        data: merchantCreateBody,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    expect(response.status()).toBe(200);
    const body = await response.json();

    this.state.set('profileId', body.default_profile);
    this.state.set('publishableKey', body.publishable_key);
    this.state.set('merchantDetails', body.merchant_details);

    console.log(`✓ Merchant created: ${randomMerchantId}`);
  }

  /**
   * Merchant Retrieve
   */
  async merchantRetrieveCall(): Promise<void> {
    const merchantId = this.state.get('merchantId');
    const response = await this.request.get(
      `${this.state.get('baseUrl')}/accounts/${merchantId}`,
      {
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
          'api-key': this.state.get('adminApiKey'),
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);

    expect(response.status()).toBe(200);
    const body = await response.json();

    expect(response.headers()['content-type']).toContain('application/json');
    expect(body.merchant_id).toBe(merchantId);
    expect(body.payment_response_hash_key).toBeTruthy();
    expect(body.publishable_key).toBeTruthy();
    expect(body.default_profile).toBeTruthy();
    expect(body.organization_id).toBeTruthy();

    this.state.set('organizationId', body.organization_id);

    if (!this.state.get('publishableKey')) {
      this.state.set('publishableKey', body.publishable_key);
    }

    console.log('✓ Merchant retrieved');
  }

  /**
   * Merchant Delete
   */
  async merchantDeleteCall(): Promise<void> {
    const merchantId = this.state.get('merchantId');
    const response = await this.request.delete(
      `${this.state.get('baseUrl')}/accounts/${merchantId}`,
      {
        headers: {
          Accept: 'application/json',
          'api-key': this.state.get('adminApiKey'),
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();
    expect(body.merchant_id).toBe(merchantId);
    expect(body.deleted).toBe(true);

    console.log('✓ Merchant deleted');
  }

  /**
   * API Key Create
   */
  async apiKeyCreateTest(apiKeyCreateBody: any): Promise<void> {
    const apiKey = this.state.get('adminApiKey');
    const baseUrl = this.state.get('baseUrl');
    const expiry = RequestBodyUtils.isoTimeTomorrow();
    const keyIdType = 'key_id';
    const keyId = RequestBodyUtils.validateEnv(baseUrl, keyIdType);
    const merchantId = this.state.get('merchantId');

    apiKeyCreateBody.expiration = expiry;

    const response = await this.request.post(
      `${baseUrl}/api_keys/${merchantId}`,
      {
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: apiKeyCreateBody,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    if (response.status() === 200) {
      const body = await response.json();

      expect(body.merchant_id).toBe(merchantId);
      expect(body.description).toBe(apiKeyCreateBody.description);
      expect(body[keyIdType]).toContain(keyId);
      expect(body[keyIdType]).toBeTruthy();

      this.state.set('apiKeyId', body.key_id);
      this.state.set('apiKey', body.api_key);

      console.log('✓ API Key created');
    } else {
      const body = await response.json();
      throw new Error(
        `API Key create call failed with status ${response.status()} and message: "${body.error?.message}"`
      );
    }
  }

  /**
   * API Key Retrieve
   */
  async apiKeyRetrieveCall(): Promise<void> {
    const merchantId = this.state.get('merchantId');
    const apiKeyId = this.state.get('apiKeyId');

    const response = await this.request.get(
      `${this.state.get('baseUrl')}/api_keys/${merchantId}/${apiKeyId}`,
      {
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
          'api-key': this.state.get('adminApiKey'),
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();
    expect(response.headers()['content-type']).toContain('application/json');
    expect(body.key_id).toBe(apiKeyId);
    expect(body.merchant_id).toBe(merchantId);

    console.log('✓ API Key retrieved');
  }

  /**
   * Customer Create
   */
  async createCustomerCall(customerCreateBody: any): Promise<void> {
    const response = await this.request.post(
      `${this.state.get('baseUrl')}/customers`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': this.state.get('apiKey'),
        },
        data: customerCreateBody,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();

    if (response.status() === 200) {
      this.state.set('customerId', body.customer_id);

      expect(body.customer_id).toBeTruthy();
      expect(body.email).toBe(customerCreateBody.email);
      expect(body.name).toBe(customerCreateBody.name);
      expect(body.phone).toBe(customerCreateBody.phone);

      if (customerCreateBody.metadata) {
        expect(body.metadata).toEqual(customerCreateBody.metadata);
      }
      if (customerCreateBody.address) {
        expect(body.address).toEqual(customerCreateBody.address);
      }
      if (customerCreateBody.phone_country_code) {
        expect(body.phone_country_code).toBe(
          customerCreateBody.phone_country_code
        );
      }

      console.log(`✓ Customer created: ${body.customer_id}`);
    } else if (response.status() === 400) {
      if (body.error?.message?.includes('already exists')) {
        expect(body.error.code).toBe('IR_12');
        expect(body.error.message).toBe(
          'Customer with the given `customer_id` already exists'
        );
      }
    }
  }

  /**
   * Customer Retrieve
   */
  async customerRetrieveCall(): Promise<void> {
    const customerId = this.state.get('customerId');

    const response = await this.request.get(
      `${this.state.get('baseUrl')}/customers/${customerId}`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': this.state.get('apiKey'),
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();
    expect(body.customer_id).toBe(customerId);
    expect(body.customer_id).toBeTruthy();

    console.log('✓ Customer retrieved');
  }

  /**
   * Get Customer (Alias for customerRetrieveCall)
   */
  async getCustomer(customerId: string): Promise<void> {
    const response = await this.request.get(
      `${this.state.get('baseUrl')}/customers/${customerId}`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': this.state.get('apiKey'),
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();
    expect(body.customer_id).toBe(customerId);
    expect(body.customer_id).toBeTruthy();

    console.log('✓ Customer retrieved');
  }

  /**
   * Update Customer
   */
  async updateCustomer(customerId: string, updateData: any): Promise<void> {
    const response = await this.request.post(
      `${this.state.get('baseUrl')}/customers/${customerId}`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': this.state.get('apiKey'),
        },
        data: updateData,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    expect(response.status()).toBe(200);
    const body = await response.json();

    expect(body.customer_id).toBe(customerId);

    // Validate updated fields in response
    if (updateData.name !== undefined) {
      expect(body.name).toBe(updateData.name);
    }
    if (updateData.email !== undefined) {
      expect(body.email).toBe(updateData.email);
    }
    if (updateData.phone !== undefined) {
      expect(body.phone).toBe(updateData.phone);
    }
    if (updateData.address !== undefined) {
      expect(body.address).toEqual(updateData.address);
    }
    if (updateData.metadata !== undefined) {
      expect(body.metadata).toEqual(updateData.metadata);
    }

    console.log(`✓ Customer updated: ${customerId}`);
  }

  /**
   * Delete Customer
   */
  async deleteCustomer(customerId: string): Promise<void> {
    const response = await this.request.delete(
      `${this.state.get('baseUrl')}/customers/${customerId}`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': this.state.get('apiKey'),
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);

    expect(response.status()).toBe(200);
    const body = await response.json();

    expect(body.customer_id).toBe(customerId);
    expect(body.deleted).toBe(true);

    // Clear customerId from state
    this.state.set('customerId', '');

    console.log(`✓ Customer deleted: ${customerId}`);
  }

  /**
   * Connector Create
   */
  async createConnectorCall(
    connectorType: string,
    createConnectorBody: any,
    paymentMethodsEnabled: any[],
    profilePrefix: string = 'profile',
    mcaPrefix: string = 'merchantConnector'
  ): Promise<void> {
    const apiKey = this.state.get('apiKey');
    const baseUrl = this.state.get('baseUrl');
    const connectorId = this.state.get('connectorId');
    const merchantId = this.state.get('merchantId');
    const profileId = this.state.get(`${profilePrefix}Id`);

    createConnectorBody.connector_type = connectorType;
    createConnectorBody.profile_id = profileId;
    createConnectorBody.connector_name = connectorId;
    createConnectorBody.payment_methods_enabled = paymentMethodsEnabled;

    // Generate unique connector_label to avoid conflicts on multiple test runs
    const connectorLabel = `${connectorId}_${RequestBodyUtils.generateRandomString('')}`;
    createConnectorBody.connector_label = connectorLabel;

    // Read connector auth file
    const connectorAuthFilePath = this.state.get('connectorAuthFilePath');
    const jsonContent = fs.readFileSync(connectorAuthFilePath, 'utf-8');
    const { authDetails, stateUpdate } = getValueByKey(
      jsonContent,
      connectorId
    );

    if (!authDetails || !authDetails.connector_account_details) {
      throw new Error(`Connector credentials not found for: ${connectorId}. Available keys: ${Object.keys(JSON.parse(jsonContent)).join(', ')}`);
    }

    if (stateUpdate) {
      this.state.set('MULTIPLE_CONNECTORS', stateUpdate.MULTIPLE_CONNECTORS);
    }

    createConnectorBody.connector_account_details =
      authDetails.connector_account_details;

    if (authDetails?.metadata) {
      createConnectorBody.metadata = {
        ...createConnectorBody.metadata,
        ...authDetails.metadata,
      };
    }

    const response = await this.request.post(
      `${baseUrl}/account/${merchantId}/connectors`,
      {
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: createConnectorBody,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    if (response.status() === 200) {
      const body = await response.json();

      expect(body.connector_name).toBe(this.state.get('connectorId'));
      this.state.set(`${mcaPrefix}Id`, body.merchant_connector_id);

      console.log(`✓ Connector created: ${body.connector_name}`);
    } else {
      const body = await response.json();
      console.error(`Response status: ${response.status()}`);
      throw new Error(`Connector Create Call Failed: ${body.error?.message}`);
    }
  }

  /**
   * Connector Retrieve
   */
  async connectorRetrieveCall(): Promise<void> {
    const merchantId = this.state.get('merchantId');
    const connectorId = this.state.get('connectorId');
    const merchantConnectorId = this.state.get('merchantConnectorId');

    const response = await this.request.get(
      `${this.state.get('baseUrl')}/account/${merchantId}/connectors/${merchantConnectorId}`,
      {
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
          'api-key': this.state.get('apiKey'),
          'x-merchant-id': merchantId,
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();
    expect(response.headers()['content-type']).toContain('application/json');
    expect(body.connector_name).toBe(connectorId);
    expect(body.merchant_connector_id).toBe(merchantConnectorId);

    console.log('✓ Connector retrieved');
  }

  /**
   * Business Profile Create
   */
  async createBusinessProfile(
    createBusinessProfile: any,
    profilePrefix: string = 'profile'
  ): Promise<void> {
    const apiKey = this.state.get('apiKey');
    const baseUrl = this.state.get('baseUrl');
    const connectorId = this.state.get('connectorId');
    const merchantId = this.state.get('merchantId');
    const profileName = `${profilePrefix}_${RequestBodyUtils.generateRandomString(connectorId)}`;

    createBusinessProfile.profile_name = profileName;

    const response = await this.request.post(
      `${baseUrl}/account/${merchantId}/business_profile`,
      {
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: createBusinessProfile,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    if (response.status() === 200) {
      const body = await response.json();
      this.state.set(`${profilePrefix}Id`, body.profile_id);
      expect(body.profile_id).toBeTruthy();

      console.log(`✓ Business profile created: ${profileName}`);
    } else {
      const body = await response.json();
      throw new Error(
        `Business Profile call failed: ${body.error?.message}`
      );
    }
  }

  /**
   * Get Business Profile
   */
  async getBusinessProfile(profileId: string): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const merchantId = this.state.get('merchantId');
    const apiKey = this.state.get('apiKey');

    const response = await this.request.get(
      `${baseUrl}/account/${merchantId}/business_profile/${profileId}`,
      {
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);

    expect(response.status()).toBe(200);
    const body = await response.json();

    expect(body.profile_id).toBe(profileId);
    expect(body.profile_name).toBeTruthy();

    console.log(`✓ Business profile retrieved: ${profileId}`);
  }

  /**
   * Update Business Profile
   */
  async updateBusinessProfile(
    profileId: string,
    updateData: any
  ): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const merchantId = this.state.get('merchantId');
    const apiKey = this.state.get('apiKey');

    const response = await this.request.post(
      `${baseUrl}/account/${merchantId}/business_profile/${profileId}`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: updateData,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    expect(response.status()).toBe(200);
    const body = await response.json();

    expect(body.profile_id).toBe(profileId);

    // Validate common update fields if provided
    if (updateData.webhook_details !== undefined) {
      expect(body.webhook_details).toEqual(updateData.webhook_details);
    }
    if (updateData.return_url !== undefined) {
      expect(body.return_url).toBe(updateData.return_url);
    }
    if (updateData.profile_name !== undefined) {
      expect(body.profile_name).toBe(updateData.profile_name);
    }
    if (updateData.payment_response_hash_key !== undefined) {
      expect(body.payment_response_hash_key).toBe(
        updateData.payment_response_hash_key
      );
    }

    console.log(`✓ Business profile updated: ${profileId}`);
  }

  /**
   * List Connectors Feature Matrix
   */
  async listConnectorsFeatureMatrix(): Promise<void> {
    const baseUrl = this.state.get('baseUrl');

    const response = await this.request.get(`${baseUrl}/feature_matrix`, {
      headers: {
        Accept: 'application/json',
      },
    });

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();
    expect(body.connectors).toBeTruthy();
    expect(Array.isArray(body.connectors)).toBe(true);
    expect(body.connectors.length).toBeGreaterThan(0);

    body.connectors.forEach((item: any) => {
      expect(item.description).toBeTruthy();
      expect(item.category).toBeTruthy();
      expect(item.integration_status).toBeTruthy();
    });

    console.log('✓ Feature matrix retrieved');
  }

  /**
   * Create Payment Intent
   * Creates a payment intent without auto-confirm
   */
  async createPaymentIntent(
    data: any,
    authType: string = 'HeaderKey'
  ): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const apiKey =
      authType === 'PublishableKey'
        ? this.state.get('publishableKey')
        : this.state.get('apiKey');

    const response = await this.request.post(`${baseUrl}/payments`, {
      headers: {
        'Content-Type': 'application/json',
        'api-key': apiKey,
      },
      data: data,
    });

    logRequestId(response.headers()['x-request-id']);

    const statusCode = response.status();
    expect([200, 201]).toContain(statusCode);

    const body = await response.json();
    this.state.set('paymentId', body.payment_id);
    this.state.set('clientSecret', body.client_secret);
    this.state.set('status', body.status);

    console.log(`✓ Payment intent created: ${body.payment_id}`);
  }

  /**
   * Confirm Payment
   * Confirms a previously created payment
   */
  async confirmPayment(
    confirmData: any,
    authType: string = 'HeaderKey'
  ): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const paymentId = this.state.get('paymentId');
    const apiKey =
      authType === 'PublishableKey'
        ? this.state.get('publishableKey')
        : this.state.get('apiKey');

    // Use client_secret from confirmData if provided, otherwise from state
    const clientSecret =
      confirmData.client_secret || this.state.get('clientSecret');
    if (clientSecret && !confirmData.client_secret) {
      confirmData.client_secret = clientSecret;
    }

    const response = await this.request.post(
      `${baseUrl}/payments/${paymentId}/confirm`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: confirmData,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();
    this.state.set('status', body.status);
    this.state.set('paymentMethod', body.payment_method);

    console.log(`✓ Payment confirmed: ${paymentId}`);
  }

  /**
   * Capture Payment
   * Captures an authorized payment
   */
  async capturePayment(
    captureData: any,
    paymentIntentId?: string,
    globalState?: State
  ): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const apiKey = this.state.get('apiKey');
    const paymentId = paymentIntentId || this.state.get('paymentId');

    const response = await this.request.post(
      `${baseUrl}/payments/${paymentId}/capture`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: captureData,
      }
    );

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();
    this.state.set('status', body.status);
    this.state.set('capturedAmount', body.amount_captured);

    console.log(`✓ Payment captured: ${paymentId}`);
  }

  /**
   * Void Payment
   * Voids/cancels a payment
   */
  async voidPayment(voidData: any, authType: string = 'HeaderKey'): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const paymentId = this.state.get('paymentId');
    const apiKey =
      authType === 'PublishableKey'
        ? this.state.get('publishableKey')
        : this.state.get('apiKey');

    const response = await this.request.post(
      `${baseUrl}/payments/${paymentId}/cancel`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: voidData,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();
    this.state.set('status', 'cancelled');

    console.log(`✓ Payment voided: ${paymentId}`);
  }

  /**
   * Retrieve Payment
   * Retrieves payment details
   */
  async retrievePayment(
    paymentId: string,
    authType: string = 'HeaderKey'
  ): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const apiKey =
      authType === 'PublishableKey'
        ? this.state.get('publishableKey')
        : this.state.get('apiKey');

    const response = await this.request.get(
      `${baseUrl}/payments/${paymentId}`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();

    console.log(`✓ Payment retrieved: ${paymentId}`);
  }

  /**
   * List Payment Methods for Customer
   */
  async listPaymentMethods(customerId?: string): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const custId = customerId || this.state.get('customerId');
    const publishableKey = this.state.get('publishableKey');

    const response = await this.request.get(
      `${baseUrl}/customers/${custId}/payment_methods`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': publishableKey,
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();
    this.state.set('paymentMethods', body.customer_payment_methods || []);

    console.log(
      `✓ Listed ${body.customer_payment_methods?.length || 0} payment methods`
    );
  }

  /**
   * List Available Payment Methods
   */
  async paymentMethodsList(data: any): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const publishableKey = this.state.get('publishableKey');

    const response = await this.request.post(
      `${baseUrl}/account/payment_methods`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': publishableKey,
        },
        data: data,
      }
    );

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();

    console.log(
      `✓ Available payment methods retrieved: ${body.payment_methods?.length || 0} methods`
    );
  }

  /**
   * List Payment Methods with Required Fields
   */
  async paymentMethodsListWithRequiredFields(data: any): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const publishableKey = this.state.get('publishableKey');

    const requestData = {
      ...data,
      required_fields: true,
    };

    const response = await this.request.post(
      `${baseUrl}/account/payment_methods`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': publishableKey,
        },
        data: requestData,
      }
    );

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();

    // Validate required_fields in response
    if (body.payment_methods && Array.isArray(body.payment_methods)) {
      body.payment_methods.forEach((pm: any) => {
        expect(pm.required_fields).toBeDefined();
      });
    }

    console.log(
      `✓ Payment methods with required fields retrieved: ${body.payment_methods?.length || 0} methods`
    );
  }

  /**
   * List Saved Payment Methods for Customer
   */
  async listCustomerPaymentMethods(customerId?: string): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const custId = customerId || this.state.get('customerId');
    const apiKey = this.state.get('apiKey');

    const response = await this.request.get(
      `${baseUrl}/customers/${custId}/payment_methods`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();

    console.log(
      `✓ Listed ${body.customer_payment_methods?.length || 0} saved payment methods for customer`
    );
  }

  /**
   * Save Card for Future Use During Payment Confirmation
   */
  async saveCardConfirm(
    confirmData: any,
    shouldFail: boolean = false
  ): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const paymentId = this.state.get('paymentId');
    const publishableKey = this.state.get('publishableKey');

    const requestData = {
      ...confirmData,
      setup_future_usage: 'on_session',
    };

    const response = await this.request.post(
      `${baseUrl}/payments/${paymentId}/confirm`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': publishableKey,
        },
        data: requestData,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    if (shouldFail) {
      expect(response.status()).not.toBe(200);
      const body = await response.json();
      console.log(
        `✓ Save card confirm failed as expected: ${body.error?.message || 'Error occurred'}`
      );
    } else {
      expect(response.status()).toBe(200);
      const body = await response.json();

      // Save payment method ID to state
      if (body.payment_method_id) {
        this.state.set('paymentMethodId', body.payment_method_id);
      }

      console.log(
        `✓ Card saved for future use: ${body.payment_method_id || 'Payment method ID not available'}`
      );
    }
  }

  /**
   * Create Mandate - Create a mandate (recurring payment authorization)
   */
  async createMandate(
    mandateData: any,
    authType: string = 'HeaderKey'
  ): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const apiKey =
      authType === 'PublishableKey'
        ? this.state.get('publishableKey')
        : this.state.get('apiKey');

    const response = await this.request.post(`${baseUrl}/payments`, {
      headers: {
        'Content-Type': 'application/json',
        'api-key': apiKey,
      },
      data: {
        ...mandateData,
        setup_future_usage: mandateData.setup_future_usage || 'off_session',
        customer_id: this.state.get('customerId'),
      },
    });

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();
    this.state.set('paymentId', body.payment_id);
    this.state.set('clientSecret', body.client_secret);
    if (body.mandate_id) {
      this.state.set('mandateId', body.mandate_id);
    }

    console.log(
      `✓ Mandate created: ${body.mandate_id || 'pending confirmation'}`
    );
  }

  /**
   * Confirm Mandate - Confirm a created mandate
   */
  async confirmMandate(confirmData: any): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const paymentId = this.state.get('paymentId');
    const apiKey = this.state.get('apiKey');

    const response = await this.request.post(
      `${baseUrl}/payments/${paymentId}/confirm`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          ...confirmData,
          client_secret: this.state.get('clientSecret'),
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();
    if (body.mandate_id) {
      this.state.set('mandateId', body.mandate_id);
    }
    this.state.set('status', body.status);

    console.log(`✓ Mandate confirmed: ${body.mandate_id}`);
  }

  /**
   * List Mandates - List customer's mandates
   */
  async listMandates(customerId: string): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const apiKey = this.state.get('apiKey');

    const response = await this.request.get(
      `${baseUrl}/customers/${customerId}/mandates`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();
    this.state.set('mandates', body);

    console.log(`✓ Mandates retrieved: ${body.length || 0} mandate(s)`);
  }

  /**
   * Revoke Mandate - Revoke an existing mandate
   */
  async revokeMandate(mandateId: string): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const apiKey = this.state.get('apiKey');

    const response = await this.request.post(
      `${baseUrl}/mandates/revoke/${mandateId}`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();
    expect(body.status).toBe('revoked');

    console.log(`✓ Mandate revoked: ${mandateId}`);
  }

  /**
   * Use Saved Payment Method - Use saved payment method with mandate
   */
  async useSavedPaymentMethod(paymentMethodId: string): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const apiKey = this.state.get('apiKey');

    const response = await this.request.post(`${baseUrl}/payments`, {
      headers: {
        'Content-Type': 'application/json',
        'api-key': apiKey,
      },
      data: {
        payment_method_id: paymentMethodId,
        recurring_enabled: true,
        customer_id: this.state.get('customerId'),
      },
    });

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();
    this.state.set('paymentId', body.payment_id);

    console.log(`✓ Saved payment method used: ${paymentMethodId}`);
  }

  /**
   * Incremental Authorization - Increment authorization amount on existing payment
   */
  async incrementalAuth(
    paymentId: string,
    incrementData: any
  ): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const apiKey = this.state.get('apiKey');

    const response = await this.request.post(
      `${baseUrl}/payments/${paymentId}/incremental_authorization`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: incrementData,
      }
    );

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();
    this.state.set('authorizedAmount', body.amount);

    console.log(`✓ Authorization incremented to ${body.amount}`);
  }

  /**
   * DDC Server-Side Race Condition - Test server-side Device Data Collection race condition
   */
  async ddcServerSideRaceCondition(
    paymentId: string,
    testData: any
  ): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const publishableKey = this.state.get('publishableKey');

    const response = await this.request.post(
      `${baseUrl}/payments/${paymentId}/3ds/authentication`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': publishableKey,
        },
        data: testData,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();
    this.state.set('ddcServerSideResponse', body);

    console.log('✓ Server-side DDC race condition test completed');
  }

  /**
   * DDC Client-Side Race Condition - Test client-side DDC race condition
   */
  async ddcClientSideRaceCondition(
    paymentId: string,
    testData: any
  ): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const publishableKey = this.state.get('publishableKey');

    const response = await this.request.post(
      `${baseUrl}/payments/${paymentId}/3ds/authentication`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': publishableKey,
        },
        data: testData,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();
    this.state.set('ddcClientSideResponse', body);

    console.log('✓ Client-side DDC race condition test completed');
  }

  /**
   * Manual Retry - Manually retry a failed payment
   */
  async manualRetry(paymentId: string, retryData: any): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const apiKey = this.state.get('apiKey');

    // Include retry flag in the data
    const requestData = {
      ...retryData,
      retry: true,
    };

    const response = await this.request.post(
      `${baseUrl}/payments/${paymentId}/confirm`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: requestData,
      }
    );

    logRequestId(response.headers()['x-request-id']);

    const body = await response.json();

    // Validate retry attempt count if available
    if (body.attempt_count !== undefined) {
      expect(body.attempt_count).toBeGreaterThan(1);
      this.state.set('retryAttemptCount', body.attempt_count);
    }

    this.state.set('paymentId', body.payment_id);
    this.state.set('status', body.status);

    console.log(`✓ Manual retry completed - Attempt: ${body.attempt_count || 'N/A'}`);
  }

  /**
   * Payment Sync - Sync payment status with connector
   */
  async paymentSync(authType: string = 'HeaderKey'): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const paymentId = this.state.get('paymentId');

    // Determine which key to use based on authType
    const authKey =
      authType === 'HeaderKey'
        ? this.state.get('apiKey')
        : this.state.get('publishableKey');

    const response = await this.request.get(
      `${baseUrl}/payments/${paymentId}/sync`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': authKey,
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();

    // Update state with latest status
    this.state.set('status', body.status);
    if (body.amount !== undefined) {
      this.state.set('amount', body.amount);
    }

    console.log(`✓ Payment synced - Status: ${body.status}`);
  }

  /**
   * Refund Sync - Sync refund status with connector
   */
  async refundSync(refundId: string): Promise<void> {
    const baseUrl = this.state.get('baseUrl');
    const apiKey = this.state.get('apiKey');

    const response = await this.request.get(
      `${baseUrl}/refunds/${refundId}/sync`,
      {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      }
    );

    logRequestId(response.headers()['x-request-id']);
    expect(response.status()).toBe(200);

    const body = await response.json();

    // Update state with current refund status
    this.state.set('refundStatus', body.status);
    if (body.amount !== undefined) {
      this.state.set('refundAmount', body.amount);
    }

    console.log(`✓ Refund synced - Status: ${body.status}`);
  }
}
