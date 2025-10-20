/**
 * Request Body Utilities for Playwright Tests
 *
 * Helper functions for manipulating request bodies and generating test data.
 * Ported from Cypress RequestBodyUtils.js
 */

interface KeyPrefixes {
  [environment: string]: {
    publishable_key: string;
    key_id: string;
  };
}

const keyPrefixes: KeyPrefixes = {
  localhost: {
    publishable_key: 'pk_dev_',
    key_id: 'dev_',
  },
  '127.0.0.1': {
    publishable_key: 'pk_dev_',
    key_id: 'dev_',
  },
  hyperswitch: {
    publishable_key: 'pk_snd_',
    key_id: 'snd_',
  },
};

/**
 * Set client secret in request body
 */
export const setClientSecret = (
  requestBody: any,
  clientSecret: string
): void => {
  requestBody.client_secret = clientSecret;
};

/**
 * Set card number in payment method data
 */
export const setCardNo = (requestBody: any, cardNo: string): void => {
  if (!requestBody.payment_method_data) {
    requestBody.payment_method_data = {};
  }
  if (!requestBody.payment_method_data.card) {
    requestBody.payment_method_data.card = {};
  }
  requestBody.payment_method_data.card.card_number = cardNo;
};

/**
 * Set API key in connector account details
 */
export const setApiKey = (requestBody: any, apiKey: string): void => {
  if (!requestBody.connector_account_details) {
    requestBody.connector_account_details = {};
  }
  requestBody.connector_account_details.api_key = apiKey;
};

/**
 * Generate random string with optional prefix
 */
export const generateRandomString = (prefix: string = 'cyMerchant'): string => {
  const uuidPart = 'xxxxxxxx';

  const randomString = uuidPart.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });

  return `${prefix}_${randomString}`;
};

/**
 * Set merchant ID in merchant create body
 */
export const setMerchantId = (
  merchantCreateBody: any,
  merchantId: string
): void => {
  merchantCreateBody.merchant_id = merchantId;
};

/**
 * Get ISO timestamp for tomorrow
 */
export function isoTimeTomorrow(): string {
  const now = new Date();
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);
  return tomorrow.toISOString();
}

/**
 * Validate environment and get key prefix
 */
export function validateEnv(baseUrl: string, keyIdType: string): string {
  if (!baseUrl) {
    throw new Error('Please provide a baseUrl');
  }

  const environment = Object.keys(keyPrefixes).find((env) =>
    baseUrl.includes(env)
  );

  if (!environment) {
    throw new Error('Unsupported baseUrl');
  }

  const envConfig = keyPrefixes[environment];
  const prefix = envConfig[keyIdType as keyof typeof envConfig];

  if (!prefix) {
    throw new Error(`Unsupported keyIdType: ${keyIdType}`);
  }

  return prefix;
}

/**
 * Generate random email address for testing
 */
export function generateRandomEmail(): string {
  const firstNames = [
    'alex', 'jamie', 'taylor', 'morgan', 'casey', 'jordan', 'pat', 'sam',
    'chris', 'dana', 'olivia', 'liam', 'emma', 'noah', 'ava', 'william',
    'sophia', 'james', 'isabella', 'oliver', 'charlotte', 'benjamin', 'amelia',
    'elijah', 'mia', 'lucas', 'harper', 'mason', 'evelyn', 'logan', 'abigail',
  ];

  const lastNames = [
    'smith', 'jones', 'williams', 'brown', 'davis', 'miller', 'wilson',
    'moore', 'taylor', 'lee', 'anderson', 'thomas', 'jackson', 'white',
    'harris', 'martin', 'garcia', 'martinez', 'robinson', 'clark', 'rodriguez',
  ];

  const domains = [
    'example.com', 'test.com', 'demo.org', 'sample.net', 'testing.io',
    'cypress.test', 'automation.dev', 'qa.example',
  ];

  const randomFirstName = firstNames[Math.floor(Math.random() * firstNames.length)];
  const randomLastName = lastNames[Math.floor(Math.random() * lastNames.length)];
  const randomDomain = domains[Math.floor(Math.random() * domains.length)];
  const randomNumber = Math.floor(Math.random() * 1000);

  return `${randomFirstName}.${randomLastName}${randomNumber}@${randomDomain}`;
}

/**
 * Generate random card holder name
 */
export function generateRandomName(): string {
  const firstNames = [
    'Alex', 'Jamie', 'Taylor', 'Morgan', 'Casey', 'Jordan', 'Pat', 'Sam',
    'Chris', 'Dana', 'Olivia', 'Liam', 'Emma', 'Noah', 'Ava', 'William',
    'Sophia', 'James', 'Isabella', 'Oliver', 'Charlotte', 'Benjamin', 'Amelia',
    'Elijah', 'Mia', 'Lucas', 'Harper', 'Mason', 'Evelyn', 'Logan', 'Abigail',
  ];

  const lastNames = [
    'Smith', 'Jones', 'Williams', 'Brown', 'Davis', 'Miller', 'Wilson',
    'Moore', 'Taylor', 'Lee', 'Dylan', 'Eleanor', 'Grayson', 'Hannah',
  ];

  const randomFirstName = firstNames[Math.floor(Math.random() * firstNames.length)];
  const randomLastName = lastNames[Math.floor(Math.random() * lastNames.length)];

  return `${randomFirstName} ${randomLastName}`;
}

/**
 * Detect if running in CI environment
 */
export const isCI = (): boolean => {
  return process.env.CI === 'true' || process.env.GITHUB_ACTIONS === 'true';
};

/**
 * Get timeout multiplier based on environment
 */
export const getTimeoutMultiplier = (): number => {
  return isCI() ? 1.5 : 1;
};
