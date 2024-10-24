
const billing = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "CA",
    zip: "94122",
    country: "US",
    first_name: "John",
    last_name: "Doe"
  }
};

const browser_info = {
  "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
  "accept_header": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
  "language": "nl-NL",
  "color_depth": 24,
  "screen_height": 723,
  "screen_width": 1536,
  "time_zone": 0,
  "java_enabled": true,
  "java_script_enabled": true,
  "ip_address": "127.0.0.1"
};

const successfulNo3DSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "10",
  card_exp_year: "2030",
  card_holder_name: "morino",
  card_cvc: "737",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "10",
  card_exp_year: "2030",
  card_holder_name: "morino",
  card_cvc: "737",
};

const payment_method_data_no3ds = {
  card: {
    last4: "4242",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "STRIPE PAYMENTS UK LIMITED",
    card_issuing_country: "UNITEDKINGDOM",
    card_isin: "424242",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "2030",
    card_holder_name: null,
    payment_checks: null,
    authentication_data: null
  },
  billing: null
};

const payment_method_data_3ds = {
  card: {
    last4: "1091",
    card_type: "Visa",
    card_network: "Visa",
    card_issuer: "INTL HDQTRS-CENTER OWNED",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "400000",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "2030",
    card_holder_name: null,
    payment_checks: null,
    authentication_data: null
  },
  billing: null
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

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      }, Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "on_session",
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method: "card",
          payment_method_type: "debit",
          attempt_count: 1,
          payment_method_data: payment_method_data_no3ds,
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          payment_method: "card",
          payment_method_type: "debit",
          attempt_count: 1,
          payment_method_data: payment_method_data_no3ds,
        },
      },
    },
    Capture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6500,
          amount_capturable: 6500,
        },
      },
    },
    PartialCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6500,
          amount_capturable: 6500,
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
      ResponseCustom: {
        body: {
          type: "invalid_request",
          message: "You cannot cancel this payment because it has status processing",
          code: "IR_16",
        }
      }
    },
    VoidAfterConfirm: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Missing required param: payment_method_data",
            code: "IR_04"
          }
        },
      },
      ResponseCustom: {
        status: 200,
        body: {
          status: "processing"
        },
      }
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        browser_info,
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Missing required param: payment_method_data",
            code: "IR_04"
          }
        },
      },
      ResponseCustom: {
        status: 200,
        body: {
          status: "processing"
        },
      }
    },
    // SaveCardUseNo3DSAutoCaptureOffSession: {
    //     Request: {
    //         payment_method: "card",
    //         payment_method_data: {
    //             card: successfulNo3DSCardDetails,
    //         },
    //         setup_future_usage: "off_session",
    //         customer_acceptance: {
    //             acceptance_type: "offline",
    //             accepted_at: "1963-05-03T04:07:52.723Z",
    //             online: {
    //                 ip_address: "127.0.0.1",
    //                 user_agent: "amet irure esse",
    //             },
    //         },
    //     },
    //     Response: {
    //         status: 200,
    //         body: {
    //             status: "processing",
    //         },
    //     },
    // },
    // SaveCardConfirmAutoCaptureOffSession: {
    //     Request: {
    //         setup_future_usage: "off_session",
    //     },
    //     Response: {
    //         status: 400,
    //         body: {
    //             type: "invalid_request",
    //             message: "Missing required param: payment_method_data",
    //             code: "IR_19",
    //         },
    //     },
    // },
    // SaveCardConfirmManualCaptureOffSession: {
    //     Request: {
    //         setup_future_usage: "off_session",
    //     },
    //     Response: {
    //         status: 400,
    //         body: {
    //             error: {
    //                 type: "invalid_request",
    //                 message: "Payment method type not supported",
    //                 code: "IR_19",
    //                 reason: "debit mandate payment is not supported by worldpay"
    //             }
    //         },
    //     },
    // },

    /**
     * Variation cases
     */
    CaptureCapturedAmount: {
      Request: {
        Request: {
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "EUR",
          customer_acceptance: null,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "This Payment could not be captured because it has a capture_method of automatic. The expected state is manual_multiple",
            code: "IR_14",
          },
        },
      },
    },
    ConfirmSuccessfulPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "You cannot confirm this payment because it has status processing",
            code: "IR_16",
          },
        },
      },
    },

    /**
     * Not implemented or not ready for running test cases
     * - 3DS
     * - Refunds
     */
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        browser_info,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: payment_method_data_3ds,
        },
      },
    },
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        browser_info,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: payment_method_data_3ds,
        },
      },
    },
    Refund: {
      Request: {},
      Response: {
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14"
          }
        }
      },
      ResponseCustom: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14",
          },
        },
      },
    },
    PartialRefund: {
      Request: {},
      Response: {
        body: {
          error: {
            type: "invalid_request",
            message: "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
            code: "IR_14"
          }
        }
      }
    },
    SyncRefund: {
      Request: {},
      Response: {
        body: {
          error: {
            type: "invalid_request",
            message: "Refund does not exist in our records.",
            code: "HE_02"
          }
        }
      }
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Worldpay is not implemented",
            code: "IR_00"
          }
        },
      },
    },
  },

  /**
   * Everything below this line is not supported by WP, but need to provide details for running the test cases
   */
}