const billing = {
  address: {
    country: "BR",
  },
};

export const connectorDetails = {
  bank_transfer_pm: {
    pix_key: {
      Create: {
        Request: {
          amount: 1500,
          currency: "BRL",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: {
              payout_method_type: "pix_key",
              pix_key: "teste_api_projeto_cobranca@santander.com.br",
            },
          },
          billing: billing,
          description: "Santander PIX payout",
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
          amount: 1500,
          currency: "BRL",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: {
              payout_method_type: "pix_key",
              pix_key: "teste_api_projeto_cobranca@santander.com.br",
            },
          },
          billing: billing,
          description: "Santander PIX payout",
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
          amount: 1500,
          currency: "BRL",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: {
              payout_method_type: "pix_key",
              pix_key: "teste_api_projeto_cobranca@santander.com.br",
            },
          },
          billing: billing,
          description: "Santander PIX payout",
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
    pix: {
      Create: {
        Request: {
          amount: 1500,
          currency: "BRL",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: {
              payout_method_type: "pix",
              bank_account_number: "12345678",
              branch_code: "1234",
              bank_code: "033",
              tax_id: "44494387100",
              account_holder_name: "John Doe",
              bank_account_type: "checking",
            },
          },
          billing: billing,
          description: "Santander PIX payout",
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
          amount: 1500,
          currency: "BRL",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: {
              payout_method_type: "pix",
              bank_account_number: "12345678",
              branch_code: "1234",
              bank_code: "033",
              tax_id: "44494387100",
              account_holder_name: "John Doe",
              bank_account_type: "checking",
            },
          },
          billing: billing,
          description: "Santander PIX payout",
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
          amount: 1500,
          currency: "BRL",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: {
              payout_method_type: "pix",
              bank_account_number: "12345678",
              branch_code: "1234",
              bank_code: "033",
              tax_id: "44494387100",
              account_holder_name: "John Doe",
              bank_account_type: "checking",
            },
          },
          billing: billing,
          description: "Santander PIX payout",
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
