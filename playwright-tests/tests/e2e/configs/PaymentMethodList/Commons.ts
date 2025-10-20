/**
 * Payment Method List Common Configuration
 *
 * Shared configuration for payment method list tests
 */

export const defaultPaymentMethodListRequest = {
  amount: 1000,
  currency: 'USD',
  recurring_enabled: true,
  installment_payment_enabled: true,
};

export const cardNetworks = [
  'AmericanExpress',
  'Discover',
  'Interac',
  'JCB',
  'Mastercard',
  'Visa',
  'DinersClub',
  'UnionPay',
  'RuPay',
];

export const expectedCardRequiredFields = [
  'payment_method_data.card.card_number',
  'payment_method_data.card.card_exp_month',
  'payment_method_data.card.card_exp_year',
  'payment_method_data.card.card_cvc',
];

export const cardCreditEnabled = [
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
    ],
  },
];
