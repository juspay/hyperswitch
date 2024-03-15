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
        "currency":"USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
        "refundSyncStatus": "succeeded"
        
    },
    "No3DS": {
        "card": successfulNo3DSCardDetails,
        "currency":"USD",
        "paymentSuccessfulStatus": "processing",
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
        "refundSyncStatus": "succeeded"
    },
    "MandateSingleUse3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency":"USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "processing",
        "refundStatus": "pending",
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
        "currency":"USD",
        "paymentSuccessfulStatus": "processing",
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
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
        "currency":"USD",
        "paymentSuccessfulStatus": "processing",
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
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
        "currency":"USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
        "refundSyncStatus": "succeeded",
        "mandate_type": {
            "multi_use": {
                "amount": 8000,
                "currency": "USD"
            }
        }
    },
};