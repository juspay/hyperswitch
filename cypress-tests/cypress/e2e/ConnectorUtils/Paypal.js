const successfulNo3DSCardDetails = {
    "card_number": "4012000033330026",
    "card_exp_month": "01",
    "card_exp_year": "25",
    "card_holder_name": "joseph Doe",
    "card_cvc": "123"

};

const successfulThreeDSTestCardDetails = {
    "card_number": "4868719460707704",
    "card_exp_month": "01",
    "card_exp_year": "25",
    "card_holder_name": "joseph Doe",
    "card_cvc": "123"
};

export const connectorDetails = {
    "3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency":"USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "processing",

    },
    "No3DS": {
        "card": successfulNo3DSCardDetails,
        "currency":"USD",
        "paymentSuccessfulStatus": "processing",
        "paymentSyncStatus": "processing",

    },
    "MandateSingleUse3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency":"USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "processing",
        "mandate_type": {
            "single_use": {
                "amount": 8000,
                "currency": "USD"
            }
        }
    },
    "MandateSingleUseNo3DS": {
        "card": successfulNo3DSCardDetails,
        "currency":"USD",
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeded",
        "mandate_type": {
            "single_use": {
                "amount": 8000,
                "currency": "USD"
            }
        }
    },
    "MandateMultiUseNo3DS": {
        "card": successfulNo3DSCardDetails,
        "currency":"USD",
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeded",
        "mandate_type": {
            "multi_use": {
                "amount": 8000,
                "currency": "USD"
            }
        }
    },
    "MandateMultiUse3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency":"USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "processing",
        "mandate_type": {
            "multi_use": {
                "amount": 8000,
                "currency": "USD"
            }
        }
    }
};