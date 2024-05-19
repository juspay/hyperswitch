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
card_pm:{
    "PaymentIntent": {
        "Request": {
            "card": successfulNo3DSCardDetails,
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
    "3DSAutoCapture": {
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
    "3DSManualCapture": {
        "Request": {
            "card": successfulThreeDSTestCardDetails,
            "currency": "USD",
            "customer_acceptance": null,
            "setup_future_usage": "on_session"
        },
        "Response": {
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "Payment method type not supported",
                    "code": "HE_03",
                    "reason": "manual is not supported by trustpay"
                }
            }
        }
    },
    "No3DSAutoCapture": {
        "Request": {
            "card": successfulNo3DSCardDetails,
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
    "No3DSManualCapture": {
        "Request": {
            "card": successfulNo3DSCardDetails,
            "currency": "USD",
            "customer_acceptance": null,
            "setup_future_usage": "on_session"
        },
        "Response": {
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "Payment method type not supported",
                    "code": "HE_03",
                    "reason": "manual is not supported by trustpay"
                }
            }
        }
    },
    "Capture": {
        "Request": {
            "card": successfulNo3DSCardDetails,
            "currency": "USD",
            "customer_acceptance": null,
        },
        "Response": {
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "This Payment could not be captured because it has a payment.status of requires_payment_method. The expected state is requires_capture, partially_captured_and_capturable, processing",
                    "code": "IR_14",
                }
            }
        }
    },
    "PartialCapture": {
        "Request": {
            "card": successfulNo3DSCardDetails,
            "currency": "USD",
            "paymentSuccessfulStatus": "succeeded",
            "paymentSyncStatus": "succeeded",
            "refundStatus": "succeeded",
            "refundSyncStatus": "succeeded",
            "customer_acceptance": null,
        },
        "Response": {
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "This Payment could not be captured because it has a payment.status of requires_payment_method. The expected state is requires_capture, partially_captured_and_capturable, processing",
                    "code": "IR_14",
                }
            }
        }
    },
    "Void":{
        "Request": {
        },
        "Response": {
            "status": 200,
            "body":{
                status: "cancelled"
    
            }
        }
    },
    "Refund": {
        "Request": {
            "card": successfulNo3DSCardDetails,
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
    "PartialRefund": {
        "Request": {
            "card": successfulNo3DSCardDetails,
            "currency": "USD",
            "customer_acceptance": null,
        },
        "Response": {
            "status": 200,
            "body": {
                "error_code": "1",
                "error_message": "transaction declined (invalid amount)",
            }

        }
    },
    "SyncRefund": {
        "Request": {
            "card": successfulNo3DSCardDetails,
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
    "MandateSingleUse3DSAutoCapture": {
        "Request": {
            "card": successfulThreeDSTestCardDetails,
            "currency": "USD",
            "mandate_type": {
                "single_use": {
                    "amount": 8000,
                    "currency": "USD"
                }
            }
        },
        "Response": {
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "Payment method type not supported",
                    "code": "HE_03",
                }
            }
        }
        
    },
    "MandateSingleUse3DSManualCapture": {
        "Request": {
            "card": successfulThreeDSTestCardDetails,
            "currency": "USD",
            "mandate_type": {
                "single_use": {
                    "amount": 8000,
                    "currency": "USD"
                }
            }
        },
        "Response": {
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "Payment method type not supported",
                    "code": "HE_03",
                }
            }
        }
        
    },
    "MandateSingleUseNo3DSManualCapture": {
        "Request": {
            "card": successfulNo3DSCardDetails,
            "currency": "USD",
            "mandate_type": {
                "single_use": {
                    "amount": 8000,
                    "currency": "USD"
                }
            }
        },
        "Response": {
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "Payment method type not supported",
                    "code": "HE_03",
                }
            }
        }
    },
    "MandateSingleUseNo3DSAutoCapture": {
        "Request": {
            "card": successfulNo3DSCardDetails,
            "currency": "USD",
            "mandate_type": {
                "single_use": {
                    "amount": 8000,
                    "currency": "USD"
                }
            }
        },
        "Response": {
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "Payment method type not supported",
                    "code": "HE_03",
                }
            }
        }
    },
    "MandateMultiUseNo3DSAutoCapture": {
        "Request": {
            "card": successfulNo3DSCardDetails,
            "currency": "USD",
            "mandate_type": {
                "multi_use": {
                    "amount": 8000,
                    "currency": "USD"
                }
            }
        },
        "Response": {
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "Payment method type not supported",
                    "code": "HE_03",
                }
            }
        }
    },
    "MandateMultiUseNo3DSManualCapture": {
        "Request": {
            "card": successfulNo3DSCardDetails,
            "currency": "USD",
            "mandate_type": {
                "multi_use": {
                    "amount": 8000,
                    "currency": "USD"
                }
            }
        },
        "Response": {
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "Payment method type not supported",
                    "code": "HE_03",
                }
            }
        }
    },
    "MandateMultiUse3DSAutoCapture": {
        "Request": {
            "card": successfulThreeDSTestCardDetails,
            "currency": "USD",
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
    "MandateMultiUse3DSManualCapture": {
        "Request": {
            "card": successfulThreeDSTestCardDetails,
            "currency": "USD",
            "mandate_type": {
                "multi_use": {
                    "amount": 8000,
                    "currency": "USD"
                }
            }
        },
        "Response": {
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "Payment method type not supported",
                    "code": "HE_03",
                }
            }
        }
    },
    "ZeroAuthMandate": {
        "Request": {
            "card": successfulNo3DSCardDetails,
            "currency": "USD",
            "mandate_type": {
                "single_use": {
                    "amount": 8000,
                    "currency": "USD"
                }
            }
        },
        "Response": {
            "status": 501,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "Setup Mandate flow for Trustpay is not implemented",
                    "code": "IR_00",
                }
            }
        }
    },
    "SaveCardUseNo3DSAutoCapture": {
        "Request": {
            "card": successfulNo3DSCardDetails,
            "currency": "USD",
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
    "SaveCardUseNo3DSManualCapture": {
        "Request": {
            "card": successfulNo3DSCardDetails,
            "currency": "USD",
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
            "status": 400,
            "body": {
                "error": {
                    "type": "invalid_request",
                    "message": "Payment method type not supported",
                    "code": "HE_03",
                }
            }
        }
    },
}
}