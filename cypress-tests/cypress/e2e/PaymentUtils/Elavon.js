const successfulNo3DSCardDetails = {
    card_number: "4111111111111111",
    card_exp_month: "06",
    card_exp_year: "25",
    card_holder_name: "joseph Doe",
    card_cvc: "123",
};

export const connectorDetails = {
    card_pm: {
        PaymentIntent: {
            Request: {
                currency: "USD",
                customer_acceptance: null,
                setup_future_usage: "on_session",
                billing: {
                    address: {
                        line1: "1467",
                        line2: "CA",
                        line3: "CA",
                        city: "Florence",
                        state: "Tuscany",
                        zip: "12345",
                        country: "IT",
                        first_name: "Max",
                        last_name: "Mustermann",
                    },
                    email: "mauro.morandi@nexi.it",
                    phone: {
                        number: "9123456789",
                        country_code: "+91",
                    },
                },
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
                payment_method: "card",
                payment_method_data: {
                    card: successfulNo3DSCardDetails,
                    billing: {
                        email: "mauro.morandi@nexi.it",
                    },
                },
                billing: {
                    email: "mauro.morandi@nexi.it",
                },
                currency: "USD",
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
                payment_method: "card",
                payment_method_data: {
                    card: successfulNo3DSCardDetails,
                    billing: {
                        email: "mauro.morandi@nexi.it",
                    },
                },
                billing: {
                    email: "mauro.morandi@nexi.it",
                },
                currency: "USD",
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
                currency: "USD",
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
        Refund: {
            Request: {
                payment_method: "card",
                payment_method_data: {
                    card: successfulNo3DSCardDetails,
                },
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
        PartialRefund: {
            Request: {
                payment_method: "card",
                payment_method_data: {
                    card: successfulNo3DSCardDetails,
                },
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
        SyncRefund: {
            Request: {
                payment_method: "card",
                payment_method_data: {
                    card: successfulNo3DSCardDetails,
                },
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
    },
};
