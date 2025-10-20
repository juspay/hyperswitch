/**
 * Common Connector Configuration Data
 *
 * Shared across all connector configs (Stripe, Cybersource, etc.)
 * Ported from Cypress Commons.js
 */

import type { CardDetails, MandateData, CustomerAcceptance } from './ConnectorTypes';
// Import connector configs statically for ES module compatibility
import { connectorDetails as stripeConfig } from './Stripe';
import { connectorDetails as cybersourceConfig } from './Cybersource';

export const customerAcceptance: CustomerAcceptance = {
  acceptance_type: 'offline',
  accepted_at: '1963-05-03T04:07:52.723Z',
  online: {
    ip_address: '127.0.0.1',
    user_agent:
      'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/22F76 [FBAN/FBIOS;FBAV/520.0.0.38.101;FBBV/756351453;FBDV/iPhone14,7;FBMD/iPhone;FBSN/iOS;FBSV/18.5;FBSS/3;FBID/phone;FBLC/fr_FR;FBOP/5;FBRV/760683563;IABMV/1]',
  },
};

export const successfulNo3DSCardDetails: CardDetails = {
  card_number: '4111111111111111',
  card_exp_month: '08',
  card_exp_year: '30',
  card_holder_name: 'joseph Doe',
  card_cvc: '999',
};

export const successfulThreeDSTestCardDetails: CardDetails = {
  card_number: '4111111111111111',
  card_exp_month: '10',
  card_exp_year: '30',
  card_holder_name: 'morino',
  card_cvc: '999',
};

export const singleUseMandateData: MandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: 'USD',
    },
  },
};

export const multiUseMandateData: MandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: 'USD',
    },
  },
};

export const cardRequiredField = {
  'payment_method_data.card.card_number': {
    required_field: 'payment_method_data.card.card_number',
    display_name: 'card_number',
    field_type: 'user_card_number',
    value: null,
  },
  'payment_method_data.card.card_exp_year': {
    required_field: 'payment_method_data.card.card_exp_year',
    display_name: 'card_exp_year',
    field_type: 'user_card_expiry_year',
    value: null,
  },
  'payment_method_data.card.card_cvc': {
    required_field: 'payment_method_data.card.card_cvc',
    display_name: 'card_cvc',
    field_type: 'user_card_cvc',
    value: null,
  },
  'payment_method_data.card.card_exp_month': {
    required_field: 'payment_method_data.card.card_exp_month',
    display_name: 'card_exp_month',
    field_type: 'user_card_expiry_month',
    value: null,
  },
};

export const payment_methods_enabled = [
  {
    payment_method: 'card',
    payment_method_types: [
      {
        payment_method_type: 'credit',
        card_networks: [
          'AmericanExpress',
          'Discover',
          'Interac',
          'JCB',
          'Mastercard',
          'Visa',
          'DinersClub',
          'UnionPay',
          'RuPay',
        ],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
      {
        payment_method_type: 'debit',
        card_networks: [
          'AmericanExpress',
          'Discover',
          'Interac',
          'JCB',
          'Mastercard',
          'Visa',
          'DinersClub',
          'UnionPay',
          'RuPay',
        ],
        minimum_amount: 0,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: true,
      },
    ],
  },
];

/**
 * Get connector-specific test configuration data
 * Returns connector config based on connector ID
 */
export function getConnectorDetails(connectorId: string): any {
  // Map connector ID to static imports
  const connectorConfigs: Record<string, any> = {
    stripe: stripeConfig,
    cybersource: cybersourceConfig,
  };

  const config = connectorConfigs[connectorId.toLowerCase()];
  if (!config) {
    throw new Error(`Unsupported connector: ${connectorId}`);
  }

  return config;
}
