const successfulNo3DSCardDetails = {
    card_number : "6011016011016011",
    card_exp_month : "10",
    card_exp_year : "2027",
    card_holder_name : "John Doe",
    card_cvc : "123",
};

export const connectorDetails = {
    card_pm : {
        PaymentIntent : {
            Request : {
                currency : "USD",
                customer_acceptance : null,
                setup_future_usage : "on_session",
            },
            Response : {
                status : 200,
                body : {
                    status : 
                    "requires_payment_method",
                },
            },
        },
        No3DSManualCapture: {
            Request: {
                currency: "USD",
                payment_method: "card",
                billing: {
                  address: {
                    line1: "1467",
                    line2: "CA",
                    line3: "CA",
                    city: "Musterhausen",
                    state: "California",
                    zip: "12345",
                    country: "US",
                    first_name: "Max",
                    last_name: "Mustermann",
                  },
                  email: "test@novalnet.de",
                  phone: {
                    number: "9123456789",
                    country_code: "+91",
                  },
                },
                payment_method_data: {
                  card: successfulNo3DSCardDetails,
                },
                customer_acceptance: null,
                setup_future_usage: "on_session",
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
              // Auto capture with different currency, so we need to pass currency in here
              currency: "USD",
              payment_method: "card",
            //   billing: {
            //     address: {
            //       line1: "1467",
            //       line2: "CA",
            //       line3: "CA",
            //       city: "Musterhausen",
            //       state: "California",
            //       zip: "12345",
            //       country: "US",
            //       first_name: "Max",
            //       last_name: "Mustermann",
            //     },
            //     email: "test@novalnet.de",
            //     phone: {
            //       number: "9123456789",
            //       country_code: "+91",
            //     },
            //   },
              payment_method_data: {
                card: successfulNo3DSCardDetails,
              },
              customer_acceptance: null,
              setup_future_usage: "on_session",
            },
            Response: {
              status: 200,
              body: {
                status: "succeeded",
              },
            },
        },
        Capture: {
            Request: {
              payment_method: "card",
              payment_method_data: {
                card: successfulNo3DSCardDetails,
              },
              customer_acceptance: null,
            },
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
        Refund: {
            Request: {
              payment_method: "card",
              payment_method_data: {
                card: successfulNo3DSCardDetails,
              },
              customer_acceptance: null,
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
              payment_method: "card",
              payment_method_data: {
                card: successfulNo3DSCardDetails,
              },
              customer_acceptance: null,
            },
            Response: {
              status: 200,
              body: {
                status: "succeeded",
              },
            },
        },
        SyncRefund: {
            Request: {
              payment_method: "card",
              payment_method_data: {
                card: successfulNo3DSCardDetails,
              },
              customer_acceptance: null,
            },
            Response: {
              status: 200,
              body: {
                status: "succeeded",
              },
            },
        },
    }
}