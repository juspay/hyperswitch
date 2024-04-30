
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

const idealBankRedirectDetails = {
    "bank_redirect": {
        "ideal": {
            "billing_details": {
                "billing_name": "John Doe"
            },
            "bank_name": "ing",
            "preferred_language": "en",
            "country": "NL"
        }
    }
}

const giropayBankRedirectDetails = {
    "bank_redirect": {
        "giropay": {
            "billing_details": {
                "billing_name": "John Doe"
            },
            "bank_name": "ing",
            "preferred_language": "en",
            "country": "DE"
        }
    }
}

const sofortBankRedirectDetails = {
    "bank_redirect": {
        "sofort": {
            "billing_details": {
                "billing_name": "John Doe"
            },
            "bank_name": "hypo_noe_lb_fur_niederosterreich_u_wien",
            "preferred_language": "en",
            "country": "NL"
        }
    }
}

const epsBankRedirectDetails = {
    "bank_redirect": {
        "eps": {
            "billing_details": {
                "billing_name": "John Doe"
            },
            "bank_name": "ing",
            "preferred_language": "en",
            "country": "AT"
        }
    }
}

export const connectorDetails = {
    "3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
        "refundSyncStatus": "pending",
        "customer_acceptance": null,
        "setup_future_usage": "on_session",
    },
    "No3DS": {
        "card": successfulNo3DSCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "succeeded",
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
        "refundSyncStatus": "pending",
        "customer_acceptance": null,
        "setup_future_usage": "on_session",
    },
    "MandateSingleUse3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "processing",
        "refundStatus": "pending",
        "refundSyncStatus": "pending",
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
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
        "refundSyncStatus": "pending",
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
        "paymentSyncStatus": "succeeded",
        "refundStatus": "pending",
        "refundSyncStatus": "pending",
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
        "paymentSyncStatus": "processing",
        "refundStatus": "pending",
        "refundSyncStatus": "pending",
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
        "paymentSyncStatus": "succeeded",
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
    "BankRedirect": {
        "3DS": {
            "ideal": {
                "payment_method": "bank_redirect",
                "payment_method_type": "ideal",
                "payment_method_data": idealBankRedirectDetails,
                "currency": "EUR",
                "customer_acceptance": null,
                "setup_future_usage": null,
                "paymentSuccessfulStatus": "requires_customer_action",
            },
            "giropay": {
                "payment_method": "bank_redirect",
                "payment_method_type": "giropay",
                "payment_method_data": giropayBankRedirectDetails,
                "currency": "EUR",
                "customer_acceptance": null,
                "setup_future_usage": null,
                "paymentSuccessfulStatus": "requires_customer_action",
            },
            "sofort": {
                "payment_method": "bank_redirect",
                "payment_method_type": "sofort",
                "payment_method_data": sofortBankRedirectDetails,
                "currency": "EUR",
                "customer_acceptance": null,
                "setup_future_usage": null,
                "paymentSuccessfulStatus": "requires_customer_action",
            },
            "eps": {
                "payment_method": "bank_redirect",
                "payment_method_type": "eps",
                "payment_method_data": epsBankRedirectDetails,
                "currency": "EUR",
                "customer_acceptance": null,
                "setup_future_usage": null,
                "paymentSuccessfulStatus": "requires_customer_action",
            }
        }
    }

}; 