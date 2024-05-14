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
card_pm:{
    "PaymentIntent": {
        "Request": {
            "card": successfulTestCardDetails,
            "currency": "USD",
            "customer_acceptance": null,
            "setup_future_usage": "on_session"
        },
        "Response": {
            "status": 200,
            "body": {
                "status": "requires_payment_method"
            }
        }
    },
    "3DS": {
        "Request": {
            "card": successfulThreeDSTestCardDetails,
            "currency": "USD",
            "customer_acceptance": null,
            "setup_future_usage": "on_session"
        },
        "Response": {
            "status": 200,
            "body": {
                "status": "succeeded"
            }
        }
    },
    "No3DS": {
        "Request": {
            "card": successfulTestCardDetails,
            "currency": "USD",
            "customer_acceptance": null,
            "setup_future_usage": "on_session"
        },
        "Response": {
            "status": 200,
            "body": {
                "status": "succeeded"
            }
        }
    },
    "Capture": {
        "Request": {
            "card": successfulTestCardDetails,
            "currency": "USD",
            "customer_acceptance": null,
        },
        "Response": {
            "status": 200,
            "body":{
                "status": "succeeded",
                "amount": 6500,
                "amount_capturable": 0,
                "amount_received": 6500,

            }
        }
    },
    "PartialCapture": {
        "Request": {
            "card": successfulTestCardDetails,
            "currency": "USD",
            "paymentSuccessfulStatus": "succeeded",
            "paymentSyncStatus": "succeeded",
            "refundStatus": "succeeded",
            "refundSyncStatus": "succeeded",
            "customer_acceptance": null,
        },
        "Response": {
            "status": 200,
            "body": {
                "status": "partially_captured",
                "amount": 6500,
                "amount_capturable": 0,
                "amount_received": 100,
            }

        }
    },
    "Refund": {
        "Request": {
            "card": successfulTestCardDetails,
            "currency": "USD",
            "customer_acceptance": null,
        },
        "Response": {
            "status": 200,
            "body": {
                "status": "succeeded",
            }

        }
    },
    "MandateSingleUse3DS": {
        "Request": {
            "card": successfulThreeDSTestCardDetails,
            "currency": "USD",
            "paymentSuccessfulStatus": "requires_customer_action",
            "paymentSyncStatus": "succeeded",
            "refundStatus": "succeeded",
            "refundSyncStatus": "succeeded",
            "mandate_type": {
                "single_use": {
                    "amount": 8000,
                    "currency": "USD"
                }
            }
        },
        "Response": {
            "status": 200,
            "body": {
                "status": "succeeded"
            }
        }
        
    },
    "MandateSingleUseNo3DS": {
        "Request": {
            "card": successfulTestCardDetails,
            "currency": "USD",
            "paymentSuccessfulStatus": "succeeded",
            "paymentSyncStatus": "succeeded",
            "refundStatus": "succeeded",
            "refundSyncStatus": "succeeded",
            "mandate_type": {
                "single_use": {
                    "amount": 8000,
                    "currency": "USD"
                }
            }
        },
        "Response": {
            "status": 200,
            "body": {
                "status": "succeeded"
             }
        }
    },
    "MandateMultiUseNo3DS": {
        "Request": {
            "card": successfulTestCardDetails,
            "currency": "USD",
            "paymentSuccessfulStatus": "succeeded",
            "paymentSyncStatus": "succeeded",
            "refundStatus": "succeeded",
            "refundSyncStatus": "succeeded",
            "mandate_type": {
                "multi_use": {
                    "amount": 8000,
                    "currency": "USD"
                }
            }
        },
        "Response": {
            "status": 200,
            "body": {
                "status": "succeeded"
            }
        }
    },
    "MandateMultiUse3DS": {
        "card": successfulThreeDSTestCardDetails,
        "currency": "USD",
        "paymentSuccessfulStatus": "requires_customer_action",
        "paymentSyncStatus": "succeeded",
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
        "Request": {
            "card": successfulTestCardDetails,
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
        "Response": {
            "status": 200,
            "body": {
                "status": "succeeded"
            }
        }
    },
}
};