export const Customer = {
  body: {
    name: "John Doe",
    email: "john.doe@example.com",
    phone: "8056594427",
    metadata: {},
  },
};

export const CreatePaymentBody = {
  profile_id: "",
  customer_id: "",
  capture_method: "automatic",
  amount: 6500,
  currency: "USD",
  confirm: true,
  capture_on: "",
  authentication_type: "no_three_ds",
  payment_method: "wallet",
  payment_method_type: "amazon_pay",
  payment_method_data: {
    wallet: {
      amazon_pay: {},
    },
  },
  billing: {
    address: {
      line1: "1467",
      line2: "Harrison Street",
      line3: "Harrison Street",
      city: "San Fransico",
      state: "California",
      zip: "94122",
      country: "US",
      first_name: "joseph",
      last_name: "Doe",
    },
    phone: {
      number: "8056594427",
      country_code: "+91",
    },
  },
};
