import { getCustomExchange } from "./Modifiers";
import { standardBillingAddress } from "./Commons";

const amazonPayShippingAddress = {
  address: {
    line1: "10 Ditka Ave",
    line2: "Suite 2500",
    line3: null,
    city: "Chicago",
    state: "IL",
    zip: "60602",
    country: "US",
    first_name: "Susie",
    last_name: "Smith",
  },
  phone: {
    number: "8000000000",
  },
};

const amazonPayDeliveryOptions = [
  {
    id: "standard-delivery",
    price: {
      amount: 2000,
      currency_code: "USD",
    },
    shipping_method: {
      shipping_method_name: "standard-courier",
      shipping_method_code: "standard-courier",
    },
    is_default: true,
  },
  {
    id: "express-delivery",
    price: {
      amount: 5000,
      currency_code: "USD",
    },
    shipping_method: {
      shipping_method_name: "express-courier",
      shipping_method_code: "express-courier",
    },
    is_default: false,
  },
];

export const connectorDetails = {
  wallet_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "USD",
        amount: 5044,
        shipping_cost: 2000,
        metadata: {
          delivery_options: amazonPayDeliveryOptions,
        },
        shipping: amazonPayShippingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    AmazonPay: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "amazon_pay",
        payment_experience: "invoke_sdk_client",
        authentication_type: "no_three_ds",
        capture_method: "automatic",
        billing: standardBillingAddress,
        shipping: amazonPayShippingAddress,
        payment_method_data: {
          wallet: {
            amazon_pay: {
              checkout_session_id: "test_checkout_session_id",
            },
          },
        },
      },
      Response: {
        status: 500,
        body: {},
      },
    }),
  },
};
