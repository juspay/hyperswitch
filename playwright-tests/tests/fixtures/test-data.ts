/**
 * Test Data Fixtures
 *
 * Request body fixtures for Playwright tests
 * Ported from Cypress fixtures/*.json
 */

export const merchantCreateBody = {
  merchant_id: 'playwright_merchant_auto',
  locker_id: 'm0010',
  merchant_name: 'NewAge Retailers',
  merchant_details: {
    primary_contact_person: 'John Test',
    primary_email: 'JohnTest@test.com',
    primary_phone: 'sunt laborum',
    secondary_contact_person: 'John Test2',
    secondary_email: 'JohnTest2@test.com',
    secondary_phone: 'cillum do dolor id',
    website: 'https://www.example.com',
    about_business:
      'Online Retail with a wide selection of organic products for North America',
    address: {
      line1: '1467',
      line2: 'Harrison Street',
      line3: 'Harrison Street',
      city: 'San Francisco',
      state: 'California',
      zip: '94122',
      country: 'US',
      first_name: 'john',
      last_name: 'Doe',
    },
  },
  webhook_details: {
    webhook_version: '1.0.1',
    webhook_username: 'ekart_retail',
    webhook_password: 'password_ekart@123',
    payment_created_enabled: true,
    payment_succeeded_enabled: true,
    payment_failed_enabled: true,
  },
  return_url: 'https://example.com',
  sub_merchants_enabled: false,
  metadata: {
    city: 'NY',
    unit: '245',
  },
  primary_business_details: [
    {
      country: 'US',
      business: 'default',
    },
  ],
};

export const merchantUpdateBody = {
  merchant_name: 'Updated Merchant Name',
  metadata: {
    city: 'SF',
    unit: '123',
  },
};

export const apiKeyCreateBody = {
  name: 'API Key 1',
  description: null,
  expiration: '2069-09-23T01:02:03.000Z',
};

export const apiKeyUpdateBody = {
  name: 'Updated API Key',
  expiration: '2069-09-23T01:02:03.000Z',
};

export const customerCreateBody = {
  email: 'guest@example.com',
  name: 'John Doe',
  phone: '999999999',
  phone_country_code: '+65',
  description: 'First customer',
  address: {
    city: 'Bangalore',
    country: 'IN',
    line1: 'Juspay router',
    line2: 'Koramangala',
    line3: 'Stallion',
    state: 'Karnataka',
    zip: '560095',
    first_name: 'John',
    last_name: 'Doe',
    origin_zip: '560095',
  },
  metadata: {
    udf1: 'value1',
    new_customer: 'true',
    login_date: '2019-09-10T10:11:12Z',
  },
};

export const customerUpdateBody = {
  name: 'Updated Customer Name',
  email: 'updated@example.com',
  phone: '888888888',
};

export const createConnectorBody = {
  connector_name: 'stripe',
  profile_id: '{{profile_id}}',
  connector_account_details: {
    auth_type: 'BodyKey',
    api_key: 'api-key',
    key1: 'value1',
  },
  test_mode: true,
  disabled: false,
  payment_methods_enabled: [],
  metadata: {
    city: 'NY',
    unit: '245',
    endpoint_prefix: 'AD',
    merchant_name: 'Playwright Test',
    account_name: 'transaction_processing',
  },
};

export const updateConnectorBody = {
  connector_name: 'stripe',
  connector_account_details: {
    auth_type: 'BodyKey',
    api_key: 'updated-api-key',
    key1: 'value1',
  },
  test_mode: true,
  disabled: false,
  metadata: {
    city: 'LA',
    unit: '999',
  },
};

export const businessProfile = {
  profile_name: 'default',
  return_url: 'https://example.com/payments',
  enable_payment_response_hash: true,
  payment_response_hash_key: 'secret_key',
  redirect_to_merchant_with_http_post: false,
  webhook_details: {
    webhook_version: '1.0.0',
  },
  metadata: {},
  routing_algorithm: null,
  intent_fulfillment_time: null,
  frm_routing_algorithm: null,
  payout_routing_algorithm: null,
};

export const createPaymentBody = {
  currency: 'USD',
  amount: 6000,
  authentication_type: 'three_ds',
  description: 'Joseph First Crypto',
  email: 'hyperswitch_sdk_demo_id@gmail.com',
  setup_future_usage: null,
  profile_id: '{{profile_id}}',
  connector_metadata: {
    noon: {
      order_category: 'applepay',
    },
  },
  metadata: {
    udf1: 'value1',
    new_customer: 'true',
    login_date: '2019-09-10T10:11:12Z',
  },
};

export const confirmBody = {
  client_secret: '',
  return_url: 'https://example.com',
  confirm: true,
  customer_acceptance: {
    acceptance_type: 'offline',
    accepted_at: '1963-05-03T04:07:52.723Z',
    online: {
      ip_address: '127.0.0.1',
      user_agent: 'amet irure esse',
    },
  },
  billing: {
    address: {
      state: 'New York',
      city: 'New York',
      country: 'US',
      first_name: 'john',
      last_name: 'doe',
      zip: '10001',
      line1: '123',
      line2: 'Main Street',
      line3: 'Apt 4B',
      origin_zip: '10001',
    },
  },
  email: 'hyperswitch_sdk_demo_id@gmail.com',
  browser_info: {
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
  },
};

export const createConfirmPaymentBody = {
  amount: 6000,
  currency: 'USD',
  confirm: true,
  capture_method: 'automatic',
  capture_on: '2022-09-10T10:11:12Z',
  customer_id: 'john123',
  email: 'guest@example.com',
  name: 'John Doe',
  phone: '999999999',
  phone_country_code: '+65',
  description: 'Its my first payment request',
  authentication_type: 'no_three_ds',
  return_url: 'https://example.com',
  setup_future_usage: 'on_session',
  customer_acceptance: {
    acceptance_type: 'offline',
    accepted_at: '1963-05-03T04:07:52.723Z',
    online: {
      ip_address: '127.0.0.1',
      user_agent: 'amet irure esse',
    },
  },
  payment_method: 'card',
  payment_method_type: 'debit',
  payment_method_data: {
    card: {
      card_number: '4242424242424242',
      card_exp_month: '01',
      card_exp_year: '50',
      card_holder_name: 'joseph Doe',
      card_cvc: '123',
    },
  },
  billing: {
    address: {
      line1: '1467',
      line2: 'Harrison Street',
      line3: 'Harrison Street',
      city: 'San Fransico',
      state: 'California',
      zip: '94122',
      country: 'US',
      first_name: 'john',
      last_name: 'doe',
    },
  },
  shipping: {
    address: {
      line1: '1467',
      line2: 'Harrison Street',
      line3: 'Harrison Street',
      city: 'San Fransico',
      state: 'California',
      zip: '94122',
      country: 'US',
      first_name: 'john',
      last_name: 'doe',
    },
  },
  statement_descriptor_name: 'joseph',
  statement_descriptor_suffix: 'JS',
  metadata: {
    udf1: 'value1',
    new_customer: 'true',
    login_date: '2019-09-10T10:11:12Z',
  },
  browser_info: {
    ip_address: '129.0.0.1',
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
  },
};

export const captureBody = {
  amount_to_capture: 100,
  statement_descriptor_name: 'Joseph',
  statement_descriptor_suffix: 'JS',
};

export const voidBody = {
  cancellation_reason: 'requested_by_customer',
};

export const refundBody = {
  payment_id: 'payment_id',
  amount: 100,
  reason: 'FRAUD',
  refund_type: 'instant',
  metadata: {
    udf1: 'value1',
    new_customer: 'true',
    login_date: '2019-09-10T10:11:12Z',
  },
};

export const listRefundCall = {
  limit: 10,
};

// Mandate-specific confirm bodies
export const citConfirmBody = {
  ...confirmBody,
  mandate_data: {
    customer_acceptance: {
      acceptance_type: 'offline',
      accepted_at: '1963-05-03T04:07:52.723Z',
      online: {
        ip_address: '127.0.0.1',
        user_agent: 'Mozilla/5.0',
      },
    },
  },
  off_session: false,
};

export const pmIdConfirmBody = {
  ...confirmBody,
  payment_method_id: '{{payment_method_id}}',
  off_session: false,
};

export const ntidConfirmBody = {
  ...confirmBody,
  payment_token: '{{network_token_id}}',
  off_session: false,
};
