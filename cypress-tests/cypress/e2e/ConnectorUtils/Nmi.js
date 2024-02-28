const successfulNo3DSCardDetails = {
    "card_number": "4000000000002503",
    "card_exp_month": "08",
    "card_exp_year": "25",
    "card_holder_name": "joseph Doe",
    "card_cvc": "999"
};

const successfulThreeDSTestCardDetails = {
    "card_number": "4000000000002503",
    "card_exp_month": "10",
    "card_exp_year": "25",
    "card_holder_name": "morino",
    "card_cvc": "999"
};

export const connectorDetails = {
    "3DS": {
        "card": successfulThreeDSTestCardDetails,
        "successfulStates": "requires_customer_action",
        "successfulSyncStates": "succeeded"
    },
    "No3DS": {
        "card": successfulNo3DSCardDetails,
        "successfulStates": "processing",
        "successfulSyncStates": "succeeded"
    },
    "MandateSingleUse3DS": {
        "card": successfulThreeDSTestCardDetails,
        "successfulStates": "requires_customer_action",
        "successfulSyncStates": "processing",
        "mandate_type": {
            "single_use": {
                "amount": 6000,
                "currency": "USD"
            }
        }
    },
    "MandateSingleUseNo3DS": {
        "card": successfulNo3DSCardDetails,
        "successfulStates": "processing",
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
        "successfulStates": "processing",
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