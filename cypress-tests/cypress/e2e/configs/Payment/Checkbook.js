export const connectorDetails = {
  bank_transfer_pm: {
    Ach: {
      Request: {
        amount: 333,
        payment_method: "bank_transfer",
        payment_method_type: "ach",
        billing: {
          address: {
            zip: "560095",
            country: "US",
            first_name: "akshakaya N",
            last_name: "sss",
            line1: "Fasdf",
            line2: "Fasdf",
            city: "Fasdf",
          },
          email: "johndoe@mail.com",
        },
        payment_method_data: {
          bank_transfer: {
            ach_bank_transfer: {},
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    MissingEmail: {
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "ach",
        billing: {
          address: {
            first_name: "John",
            last_name: "Doe",
          },
        },
        currency: "USD",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "email is a required field",
            code: "IR_01",
          },
        },
      },
    },
  },
};
