// This file is the default. To override, add to connector.js
import { getCustomExchange } from "./Modifiers";

const card_data = {
  card_number: "4111111111111111",
  expiry_month: "3",
  expiry_year: "2030",
  card_holder_name: "John Smith",
};

const payment_card_data = {
  card_number: "4111111111111111",
  card_exp_month: "03",
  card_exp_year: "2030",
  card_holder_name: "John Doe",
};

const billing = {
  address: {
    line1: "Raadhuisplein",
    line2: "92",
    city: "Hoogeveen",
    state: "FL",
    zip: "7901 BW",
    country: "NL",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+31",
  },
};

export const connectorDetails = {
  card_pm: {
    Create: getCustomExchange({
      Request: {
        payout_type: "card",
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
          payout_type: "card",
        },
      },
    }),
    Confirm: getCustomExchange({
      Request: {
        payout_type: "card",
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "card",
        },
      },
    }),
    Fulfill: getCustomExchange({
      Request: {
        payout_type: "card",
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
      },
    }),
    SavePayoutMethod: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        card: payment_card_data,
        metadata: {
          city: "NY",
          unit: "245",
        },
      },
      Response: {
        status: 200,
      },
    }),
    Token: getCustomExchange({
      Request: {
        payout_token: "token",
        payout_type: "card",
      },
    }),
  },
  bank_transfer_pm: {
    Create: getCustomExchange({
      Request: {
        payout_type: "bank",
        priority: "regular",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
          payout_type: "bank",
        },
      },
    }),
    Confirm: getCustomExchange({
      Request: {
        payout_type: "bank",
        priority: "regular",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "bank",
        },
      },
    }),
    Fulfill: getCustomExchange({
      Request: {
        payout_type: "bank",
        priority: "regular",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
    }),
    Token: getCustomExchange({
      Request: {
        payout_token: "token",
        payout_type: "card",
      },
    }),
  },
  payout_link_pm: {
    PayoutLinkBasic: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "USD",
        amount: 100,
        description: "Test Payout Link",
        payout_link_config: {
          test_mode: true,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
        },
      },
    }),
    PayoutLinkWithTheme: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "USD",
        amount: 100,
        description: "Test with custom theme",
        payout_link_config: {
          test_mode: true,
          theme: "#FF6B35",
        },
      },
      Response: {
        status: 200,
      },
    }),
    PayoutLinkWithLogo: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "EUR",
        amount: 100,
        description: "Test with merchant logo",
        payout_link_config: {
          test_mode: true,
          logo: "https://example.com/logo.png",
          merchant_name: "Test Merchant Inc",
        },
      },
      Response: {
        status: 200,
      },
    }),
    PayoutLinkWithSdkLayout: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "GBP",
        amount: 100,
        description: "Test with accordion layout",
        payout_link_config: {
          test_mode: true,
          sdk_layout: "accordion",
        },
      },
      Response: {
        status: 200,
      },
    }),
    PayoutLinkTabsLayout: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "EUR",
        amount: 100,
        description: "Test with tabs layout",
        payout_link_config: {
          test_mode: true,
          sdk_layout: "tabs",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
        },
      },
    }),
    PayoutLinkBankTransfer: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "USD",
        amount: 100,
        description: "Test Payout Link Bank Transfer",
        payout_link_config: {
          test_mode: true,
          enabled_payment_methods: ["bank_transfer"],
        },
      },
      Response: {
        status: 200,
      },
      BankData: {
        account_number: "000123456",
        routing_number: "110000000",
        bank_name: "Test Bank",
      },
    }),
    PayoutLinkValidationError: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "USD",
        amount: 100,
        description: "Test missing customer_id",
        customer_id: null,
        payout_link_config: {
          test_mode: true,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_04",
            message:
              "Provide either customer or customer_id when payout_link is true",
          },
        },
      },
    }),
    PayoutLinkConfirmConflict: getCustomExchange({
      Request: {
        payout_link: true,
        confirm: true,
        currency: "USD",
        amount: 100,
        description: "Test confirm + payout_link conflict",
        payout_link_config: {
          test_mode: true,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_04",
            message: "Cannot confirm a payout while creating a payout link",
          },
        },
      },
    }),
    PayoutLinkWithoutLink: getCustomExchange({
      Request: {
        payout_link: false,
        currency: "USD",
        amount: 100,
        description: "Test without payout link",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payout_method_data",
        },
      },
    }),
    PayoutLinkProfileConfig: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "USD",
        amount: 100,
        description: "Test profile-level payout link config",
        payout_link_config: {
          test_mode: true,
        },
      },
      Response: {
        status: 200,
      },
    }),
    PayoutLinkCustomId: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "USD",
        amount: 100,
        description: "Test custom payout link id",
        payout_link_config: {
          test_mode: true,
          payout_link_id: "custom_payout_link_123",
        },
      },
      Response: {
        status: 200,
      },
    }),
    PayoutLinkCardPayment: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "USD",
        amount: 100,
        description: "Test Payout Link Card Payment",
        payout_link_config: {
          test_mode: true,
          enabled_payment_methods: ["card"],
        },
      },
      Response: {
        status: 200,
      },
      CardData: {
        card_number: "4242424242424242",
        card_exp_month: "12",
        card_exp_year: "35",
        card_cvc: "123",
      },
    }),
    PayoutLinkInvalidCard: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "USD",
        amount: 100,
        description: "Test Payout Link Invalid Card",
        payout_link_config: {
          test_mode: true,
          enabled_payment_methods: ["card"],
        },
      },
      Response: {
        status: 200,
      },
      CardData: {
        card_number: "4000000000000002",
        card_exp_month: "12",
        card_exp_year: "35",
        card_cvc: "123",
      },
    }),
    PayoutLinkExpiredCard: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "USD",
        amount: 100,
        description: "Test Payout Link Expired Card",
        payout_link_config: {
          test_mode: true,
          enabled_payment_methods: ["card"],
        },
      },
      Response: {
        status: 200,
      },
      CardData: {
        card_number: "4000000000000069",
        card_exp_month: "12",
        card_exp_year: "20",
        card_cvc: "123",
      },
    }),
  },
};
