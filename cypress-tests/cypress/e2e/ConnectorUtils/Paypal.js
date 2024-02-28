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
        "successfulStates": "requires_customer_action"
    },
    "No3DS": {
        "card": successfulNo3DSCardDetails,
        "successfulStates": "processing"
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
        "successfulStates": "requires_customer_action",
        "successfulSyncStates": "processing",
        "mandate_type": {
            "multi_use": {
                "amount": 6000,
                "currency": "USD"
            }
        }
    },

}; 