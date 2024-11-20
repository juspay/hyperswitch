const billing = {
    address: {
        line1: "1467",
        line2: "Harrison Street",
        line3: "Harrison Street",
        city: "Munich",
        state: "CA",
        zip: "80331",
        country: "DE",
        first_name: "John",
        last_name: "Doe"
    },
    phone: {
        number: "9123456789",
        country_code: "+91"
    },
    email: `payout_customer${Date.now()}@example.com`
};

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

const bank = {
    iban: "DE57331060435647542639",
    bic: "DEUTDE5M551",
    bank_name: "Deutsche Bank",
    bank_country_code: "DE",
    bank_city: "Munich"
};

const error = {
  type: "invalid_request",
  message: `Payout Eligibility for Nomupay is not implemented`,
  code: "IR_00",
}

export const connectorDetails = {
    card_pm: {
      Create: {
        Request: {
          payout_method_data: {
            card: card_data,
          },
          payout_type: "card",
        },
        Response: {
          status: 501,
          body: {
            error: error,
          },
        },
      },
      Confirm: {
        Request: {
          payout_method_data: {
            card: card_data,
          },
          payout_type: "card",
        },
        Response: {
          status: 501,
          body: {
            error: error,
          },
        },
      },
      Fulfill: {
        Request: {
          payout_method_data: {
            card: card_data,
          },
          currency: "EUR",
          payout_type: "card",
          recurring: true,
        },
        Response: {
          status: 501,
          body: {
            error: error,
          },
        },
      },
      SavePayoutMethod: {
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
      },
      Token: {
        Request: {
          payout_token: "token",
          payout_type: "card",
        },
        Response: {
          status: 501,
          body: {
            error: error,
          },
        },
      },
    },
    bank_transfer_pm: {
      sepa: {
        Create: {
          Request: {
            currency: "EUR",
            payout_type: "bank",
            payout_method_data: bank,
            billing: billing,
          },
          Response: {
            status: 200,
            body: {
              status: "requires_confirmation",
              payout_type: "bank",
            },
          },
        },
        Confirm: {
          Request: {
            payout_type: "bank",
            payout_method_data: bank,
            billing: billing,
          },
          Response: {
            status: 200,
            body: {
              status: "requires_fulfillment",
              payout_type: "bank",
            },
          },
        },
        Fulfill: {
          Request: {
            currency: "EUR",
            payout_type: "bank",
            payout_method_data: bank,
            billing: billing,
            recurring: true,
          },
          Response: {
            status: 200,
            body: {
              status: "success",
              payout_type: "bank",
            },
          },
        },
        SavePayoutMethod: {
          Request: {
            payment_method: "bank_transfer",
            payment_method_type: "sepa",
            bank_transfer: bank,
            
          },
          Response: {
            status: 200,
            body: {
              payment_method: "bank_transfer",
              payment_method_type: "sepa",
            },
          },
        },
        Token: {
          Request: {
            payout_token: "token",
            payout_type: "bank",
          },
          Response: {
            status: 200,
            body: {
              status: "success",
              payout_type: "bank",
            },
          },
        },
      },
    },
  };
  