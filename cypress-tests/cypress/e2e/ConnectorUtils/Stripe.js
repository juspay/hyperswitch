const successfulTestCard = "4242424242424242";
const successful3DSCard = "4000002760003184";

const successfulTestCardDetails = {
    "card_number": "4242424242424242",
    "card_exp_month": "10",
    "card_exp_year": "25",
    "card_holder_name": "morino",
    "card_cvc": "737"
};

const successfulThreeDSTestCardDetails = {
    "card_number": "4000002760003184",
    "card_exp_month": "10",
    "card_exp_year": "25",
    "card_holder_name": "morino",
    "card_cvc": "737"
};

export const connectorDetails = {
    "3DS": {
        "card": successfulThreeDSTestCardDetails,
        "successfulStates": "requires_customer_action",
        "successfulSyncStates": "succeeded"
    },
    "No3DS": {
        "card": successfulTestCardDetails,
        "successfulStates": "succeeded",
        "successfulSyncStates": "succeeded"
    },
    "MandateSingleUse3DS": {
        "card": successfulThreeDSTestCardDetails,
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
        "card": successfulTestCardDetails,
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
        "card": successfulTestCardDetails,
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
        "successfulSyncStates": "succeeded",
        "mandate_type": {
            "multi_use": {
                "amount": 6000,
                "currency": "USD"
            }
        }
    },


};