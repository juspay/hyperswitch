
const successfulNo3DSCardDetails = {
    "card_number": "371449635398431",
    "card_exp_month": "03",
    "card_exp_year": "30",
    "card_holder_name": "John Doe",
    "card_cvc": "7373"
};

const successfulThreeDSTestCardDetails = {
    "card_number": "4917610000000000",
    "card_exp_month": "03",
    "card_exp_year": "30",
    "card_holder_name": "Joseph Doe",
    "card_cvc": "737"
};

export const connectorDetails = {
    "3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency":"USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
        "refundSyncStatus": "pending"
    },
    "No3DS": {
        "card": successfulNo3DSCardDetails,
        "currency":"USD",
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
        "refundSyncStatus": "pending"
    },
    "MandateSingleUse3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency":"USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "processing",
        "refundStatus": "pending",
        "refundSyncStatus": "pending",
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
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
        "refundSyncStatus": "pending",
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
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
        "refundSyncStatus": "pending",
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
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "processing",
        "refundStatus": "pending",
        "refundSyncStatus": "pending",
        "mandate_type": {
            "multi_use": {
                "amount": 6000,
                "currency": "USD"
            }
        }
    },
}; 