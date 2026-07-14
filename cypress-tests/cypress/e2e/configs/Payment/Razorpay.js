export const connectorDetails = {
  upi_pm: {
    // Razorpay deprecated UPI Collect flow on 28 Feb 2026 per NPCI guidelines.
    // Standard merchants can no longer use UPI Collect (VPA-based) payments.
    // TRIGGER_SKIP on PaymentIntent ensures the entire UPI Collect test flow
    // is cleanly skipped — createPaymentIntentTest has an early return for
    // TRIGGER_SKIP, so no API call is made and all downstream steps skip.
    // Ref: https://razorpay.com/docs/payments/payment-methods/upi/
    PaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "INR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    UpiCollect: {
      Request: {
        payment_method: "upi",
        payment_method_type: "upi_collect",
        payment_method_data: {
          upi: {
            upi_collect: {
              vpa_id: "successtest@iata",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Mumbai",
            state: "Maharashtra",
            zip: "400001",
            country: "IN",
            first_name: "john",
            last_name: "doe",
          },
          phone: {
            number: "9999999999",
            country_code: "+91",
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
    Refund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
  },
};
