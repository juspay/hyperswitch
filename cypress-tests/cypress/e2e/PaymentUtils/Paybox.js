const successfulNo3DSCardDetails = {
    card_number: "1111222233334444",
    card_exp_month: "05",
    card_exp_year: "27",
    card_holder_name: "joseph Doe",
    card_cvc: "222",
};

export const connectorDetails = {
    card_pm: {
        PaymentIntent: {
            Request: {
                payment_method: "card",
                payment_method_data: {
                    card: successfulNo3DSCardDetails,
                },
                currency: "EUR",
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
                payment_method: "card",
                payment_method_data: {
                    card: successfulNo3DSCardDetails,
                },
                customer_acceptance: null,
                setup_future_usage: "on_session",
            },
            Response: {
                status: 200,
                body: {
                    status: "processing",
                },
            },
        },
        No3DSAutoCapture: {
            Request: {
                payment_method: "card",
                payment_method_data: {
                    card: successfulNo3DSCardDetails,
                },
                customer_acceptance: null,
                setup_future_usage: "on_session",
            },
            Response: {
                status: 200,
                body: {
                    status: "processing",
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
                    status: "processing",
                    amount: 6500,
                    amount_capturable: 6500,
                    amount_received: null,
                },
            },
        },
        PartialCapture: {
            Request: {},
            Response: {
                status: 200,
                body: {
                    status: "processing",
                    amount: 6500,
                    amount_capturable: 6500,
                    amount_received: null,
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
                    status: "pending",
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
                    status: "pending",
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
    },
};