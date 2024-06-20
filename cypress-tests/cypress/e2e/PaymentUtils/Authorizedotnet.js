const successfulNo3DSCardDetails = {
    card_number: "5424000000000015",
    card_exp_month: "01",
    card_exp_year: "35",
    card_holder_name: "Joseph Doe",
    card_cvc: "123",
  };
  
  const successfulThreeDSTestCardDetails = {
    card_number: "4917610000000000",
    card_exp_month: "03",
    card_exp_year: "30",
    card_holder_name: "Joseph Doe",
    card_cvc: "737",
  };
  
  const singleUseMandateData = {
    customer_acceptance: {
      acceptance_type: "offline",
      accepted_at: "1963-05-03T04:07:52.723Z",
      online: {
        ip_address: "125.0.0.1",
        user_agent: "amet irure esse",
      },
    },
    mandate_type: {
      single_use: {
        amount: 8000,
        currency: "USD",
      },
    },
  };
  
  const multiUseMandateData = {
    customer_acceptance: {
      acceptance_type: "offline",
      accepted_at: "1963-05-03T04:07:52.723Z",
      online: {
        ip_address: "125.0.0.1",
        user_agent: "amet irure esse",
      },
    },
    mandate_type: {
      multi_use: {
        amount: 8000,
        currency: "USD",
      },
    },
  };
  
  export const connectorDetails = {
    card_pm: {
      PaymentIntent: {
        Request: {
          card: successfulNo3DSCardDetails,
          currency: "USD",
          customer_acceptance: null,
          setup_future_usage: "on_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      },
      No3DSManualCapture: {
        Request: {
          card: successfulNo3DSCardDetails,
          currency: "USD",
          customer_acceptance: null,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_capture",
          },
        },
      },
      No3DSAutoCapture: {
        Request: {
          card: successfulNo3DSCardDetails,
          currency: "USD",
          customer_acceptance: null,
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      Capture: {
        Request: {},
        Response: {
          status: 200,
          body: {
            status: "succeeded",
            amount: 6500,
            amount_capturable: 0,
            amount_received: 6500,
          },
        },
      },
      PartialCapture: {
        Request: {},
        Response: {
          status: 200,
          body: {
            status: "partially_captured",
            amount: 6500,
            amount_capturable: 0,
            amount_received: 100,
          },
        },
      },
      Void: {
        Request: {},
        Response: {
          status: 200,
          body: {
            status: "cancelled",
          },
        },
      },
      Refund: {
        Request: {
          currency: "USD",
          customer_acceptance: null,
        },
        Response: {
          status: 200,
          body: {
            status: "failed",
            error_message: "The referenced transaction does not meet the criteria for issuing a credit.",
            error_code: "54",
          },
        },
      },
      PartialRefund: {
        Request: {
          currency: "USD",
          customer_acceptance: null,
        },
        Response: {
          status: 200,
          body: {
            status: "failed",
            error_message: "The referenced transaction does not meet the criteria for issuing a credit.",
            error_code: "54",
          },
        },
      },
      SyncRefund: {
        Request: {
          card: successfulNo3DSCardDetails,
          currency: "USD",
          customer_acceptance: null,
        },
        Response: {
          status: 200,
          body: {
            status: "pending",
          },
        },
      },
      MandateSingleUse3DSAutoCapture: {
        Request: {
          payment_method_data: {
            card: successfulThreeDSTestCardDetails,
          },
          currency: "USD",
          mandate_data: singleUseMandateData
          },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      MandateSingleUse3DSManualCapture: {
        Request: {
          payment_method_data: {
            card: successfulThreeDSTestCardDetails,
          },
          currency: "USD",
          mandate_data: singleUseMandateData
        },
        Response: {
          status: 200,
          body: {
            status: "requires_customer_action",
          },
        },
      },
      MandateSingleUseNo3DSAutoCapture: {
        Request: {
           payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "USD",
          mandate_data: singleUseMandateData
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      MandateSingleUseNo3DSManualCapture: {
        Request: {
            payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "USD",
          mandate_data: singleUseMandateData
        },
        Response: {
          status: 200,
          body: {
            status: "requires_capture",
          },
        },
      },
      MandateMultiUseNo3DSAutoCapture: {
        Request: {
            payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "USD",
          mandate_data: multiUseMandateData
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      MandateMultiUseNo3DSManualCapture: {
        Request: {
            payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "USD",
          mandate_data: multiUseMandateData
        },
        Response: {
          status: 200,
          body: {
            status: "requires_capture",
          },
        },
      },
      MandateMultiUse3DSAutoCapture: {
        Request: {
          payment_method_data: {
            card: successfulThreeDSTestCardDetails,
          },
          currency: "USD",
          mandate_data: multiUseMandateData
        },
        Response: {
          status: 200,
          body: {
            status: "requires_capture",
          },
        },
      },
      MandateMultiUse3DSManualCapture: {
        Request: {
          payment_method_data: {
            card: successfulThreeDSTestCardDetails,
          },
          currency: "USD",
          mandate_data: multiUseMandateData
        },
        Response: {
          status: 200,
          body: {
            status: "requires_capture",
          },
        },
      },
      ZeroAuthMandate: {
        Request: {
           payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "USD",
          mandate_data: singleUseMandateData
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      SaveCardUseNo3DSAutoCapture: {
        Request: {
          card: successfulNo3DSCardDetails,
          currency: "USD",
          setup_future_usage: "on_session",
          customer_acceptance: {
            acceptance_type: "offline",
            accepted_at: "1963-05-03T04:07:52.723Z",
            online: {
              ip_address: "127.0.0.1",
              user_agent: "amet irure esse",
            },
          },
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      SaveCardUseNo3DSManualCapture: {
        Request: {
          card: successfulNo3DSCardDetails,
          currency: "USD",
          setup_future_usage: "on_session",
          customer_acceptance: {
            acceptance_type: "offline",
            accepted_at: "1963-05-03T04:07:52.723Z",
            online: {
              ip_address: "127.0.0.1",
              user_agent: "amet irure esse",
            },
          },
        },
        Response: {
          status: 200,
          body: {
            status: "requires_capture",
          },
        },
      },
      PaymentMethodIdMandateNo3DSAutoCapture: {
        Request: {
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "USD",
          mandate_data: null,
          customer_acceptance: {
            acceptance_type: "offline",
            accepted_at: "1963-05-03T04:07:52.723Z",
            online: {
              ip_address: "125.0.0.1",
              user_agent: "amet irure esse",
            },
          },
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
          },
        },
      },
      PaymentMethodIdMandateNo3DSManualCapture: {
        Request: {
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "USD",
          mandate_data: null,
          customer_acceptance: {
            acceptance_type: "offline",
            accepted_at: "1963-05-03T04:07:52.723Z",
            online: {
              ip_address: "125.0.0.1",
              user_agent: "amet irure esse",
            },
          },
        },
        Response: {
          status: 200,
          body: {
            status: "requires_capture",
          },
        },
      },
      PaymentMethodIdMandate3DSAutoCapture: {
        Request: {
          payment_method_data: {
            card: successfulThreeDSTestCardDetails,
          },
          currency: "USD",
          mandate_data: null,
          authentication_type: "three_ds",
          customer_acceptance: {
            acceptance_type: "offline",
            accepted_at: "1963-05-03T04:07:52.723Z",
            online: {
              ip_address: "125.0.0.1",
              user_agent: "amet irure esse",
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
      PaymentMethodIdMandate3DSManualCapture: {
        Request: {
          payment_method_data: {
            card: successfulThreeDSTestCardDetails,
          },
          mandate_data: null,
          authentication_type: "three_ds",
          customer_acceptance: {
            acceptance_type: "offline",
            accepted_at: "1963-05-03T04:07:52.723Z",
            online: {
              ip_address: "125.0.0.1",
              user_agent: "amet irure esse",
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
    },
  };
  