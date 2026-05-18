export const connectorDetails = {
  bank_redirect_pm: {
    OpenBankingUk: {
      Request: {
        payment_method: "bank_redirect",
        amount: 6000,
        currency: "GBP",
        payment_method_type: "open_banking_uk",
        payment_method_data: {
          bank_redirect: {
            open_banking_uk: {
              issuer: "citi",
              country: "GB",
            },
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
            country: "GB",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+44",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "open_banking_uk",
          payment_method_type_display_name: "Open Banking",
          connector: "volt",
        },
      },
      Refund: {
        Request: {
          amount: 6000,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      PartialRefund: {
        Request: {
          amount: 2000,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      manualPaymentRefund: {
        Request: {
          amount: 6000,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      manualPaymentPartialRefund: {
        Request: {
          amount: 2000,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      SyncRefund: {
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
    },
    OpenBanking: {
      Request: {
        payment_method: "bank_redirect",
        amount: 6000,
        currency: "EUR",
        payment_method_type: "open_banking",
        payment_method_data: {
          bank_redirect: {
            open_banking: {},
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
            country: "DE",
            first_name: "John",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+49",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "open_banking",
          payment_method_type_display_name: "Open Banking",
          connector: "volt",
        },
      },
      Refund: {
        Request: {
          amount: 6000,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      PartialRefund: {
        Request: {
          amount: 2000,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      manualPaymentRefund: {
        Request: {
          amount: 6000,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      manualPaymentPartialRefund: {
        Request: {
          amount: 2000,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      SyncRefund: {
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
    },
  },
};
