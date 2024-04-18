const successfulNo3DSCardDetails = {
    "card_number": "4200000000000000",
    "card_exp_month": "10",
    "card_exp_year": "25",
    "card_holder_name": "joseph Doe",
    "card_cvc": "123"
};

const successfulThreeDSTestCardDetails = {
    "card_number": "4200000000000067",
    "card_exp_month": "03",
    "card_exp_year": "2030",
    "card_holder_name": "John Doe",
    "card_cvc": "737",
};

export const connectorDetails = {
    "3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "succeeed",
        "voidStatus": "cancelled",
        "refundStatus": "succeeded",
        "refundSyncStatus": "succeeded",
        "customer_acceptance": null,
        "setup_future_usage": "on_session",
    },
    "No3DS": {
        "card": successfulNo3DSCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeed",
        "voidStatus": "cancelled",
        "refundStatus": "succeeded",
        "refundSyncStatus": "succeeded",
        "customer_acceptance": null,
        "setup_future_usage": "on_session",
    },
    "No3DSManual": {
        "card": successfulNo3DSCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeed",
        "voidStatus": "cancelled",
        "refundStatus": "succeeded",
        "refundSyncStatus": "succeeded",
        "customer_acceptance": null,
        "setup_future_usage": "on_session",
    },
    "MandateSingleUse3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "succeeed",
        "voidStatus": "cancelled",
        "refundStatus": "succeeded",
        "refundSyncStatus": "succeeded",
        "mandate_type": {
            "single_use": {
                "amount": 8000,
                "currency": "USD"
            }
        }
    },
    "MandateSingleUseNo3DS": {
        "card": successfulNo3DSCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeed",
        "voidStatus": "cancelled",
        "refundStatus": "succeeded",
        "refundSyncStatus": "succeeded",
        "mandate_type": {
            "single_use": {
                "amount": 8000,
                "currency": "USD"
            }
        }
    },
    "MandateMultiUseNo3DS": {
        "card": successfulNo3DSCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeed",
        "voidStatus": "cancelled",
        "refundStatus": "succeeded",
        "refundSyncStatus": "succeeded",
        "mandate_type": {
            "multi_use": {
                "amount": 8000,
                "currency": "USD"
            }
        }
    },
    "MandateSingleUseNo3DSManual": {
        "card": successfulNo3DSCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeed",
        "voidStatus": "cancelled",
        "refundStatus": "succeeded",
        "refundSyncStatus": "succeeded",
        "mandate_type": {
            "single_use": {
                "amount": 8000,
                "currency": "USD"
            }
        }
    },
    "MandateMultiUseNo3DSManual": {
        "card": successfulNo3DSCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeed",
        "voidStatus": "cancelled",
        "refundStatus": "succeeded",
        "refundSyncStatus": "succeeded",
        "mandate_type": {
            "multi_use": {
                "amount": 8000,
                "currency": "USD"
            }
        }
    },
    "MandateMultiUse3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "succeeed",
        "voidStatus": "cancelled",
        "refundStatus": "succeeded",
        "refundSyncStatus": "succeeded",
        "mandate_type": {
            "multi_use": {
                "amount": 8000,
                "currency": "USD"
            }
        }
    },
    "SaveCardUseNo3DS": {
        "card": successfulNo3DSCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeed",
        "voidStatus": "cancelled",
        "refundStatus": "succeeded",
        "refundSyncStatus": "succeeded",
        "setup_future_usage": "on_session",
        "customer_acceptance": {
            "acceptance_type": "offline",
            "accepted_at": "1963-05-03T04:07:52.723Z",
            "online": {
                "ip_address": "127.0.0.1",
                "user_agent": "amet irure esse"
            }
        },
    },
};
