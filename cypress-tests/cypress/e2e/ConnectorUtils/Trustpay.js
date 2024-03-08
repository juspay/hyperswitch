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
        "currency":"USD",
        "successfulStates": "requires_customer_action",
        "successfulSyncStates": "succeeded"
    },
    "No3DS": {
        "card": successfulNo3DSCardDetails,
        "currency":"USD",
        "successfulStates": "succeeded",
        "successfulSyncStates": "succeeded"
    },
    "MandateSingleUse3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency":"USD",
        "successfulStates": "requires_customer_action",
        "successfulSyncStates": "succeeded",
        "mandate_type": {
            "single_use": {
                "amount": 6000,
                "currency": "USD"
            }
        }
    },
    "MandateSingleUseNo3DS": {
        "card": successfulNo3DSCardDetails,
        "currency":"USD",
        "successfulStates": "succeeded",
        "successfulSyncStates": "succeeded",
        "mandate_type": {
            "single_use": {
                "amount": 6000,
                "currency": "USD"
            }
        }
    },
    "MandateMultiUseNo3DS": {
        "card": successfulNo3DSCardDetails,
        "currency":"USD",
        "successfulStates": "succeeded",
        "successfulSyncStates": "succeeded",
        "mandate_type": {
            "multi_use": {
                "amount": 6000,
                "currency": "USD"
            }
        }
    },
    "MandateMultiUse3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency":"USD",
        "successfulStates": "requires_customer_action",
        "successfulSyncStates": "succeeded",
        "mandate_type": {
            "multi_use": {
                "amount": 6000,
                "currency": "USD"
            }
        }
    },
};