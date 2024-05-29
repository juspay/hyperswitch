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
        "3DSManualCapture": {
            "Request": {
                "card": successfulThreeDSTestCardDetails,
                "currency": "USD",
                "customer_acceptance": null,
                "setup_future_usage": "on_session"
            },
            "Response": {
                "status": 200,
                "body": {
                    "status": "requires_capture"
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
                    "status": "processing"
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
                "status": 200,
                "body": {
                    "status": "requires_capture"
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
                    "status": "processing"
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
                "status": 200,
                "body":{
                    "status": "processing",
                    "amount": 6500,
                    "amount_capturable": 6500,
                    "amount_received": 0,
    
                }
            }
        },
        "PartialCapture": {
            "Request": {
             
            },
            "Response": {
                "status": 200,
                "body": {
                    "status": "processing",
                    "amount": 6500,
                    "amount_capturable": 6500,
                    "amount_received": 0,
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
                "status": 400,
                "body": {
                    "error": {
                        "type": "invalid_request",
                        "message": "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
                        "code"   : "IR_14"
                    },
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
                "status": 400,
                "body": {
                    "error": {
                        "type": "invalid_request",
                        "message": "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
                        "code"   : "IR_14"
                    },
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
                "status": 400,
                "body": {
                    "error": {
                        "type": "invalid_request",
                        "message": "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
                        "code"   : "IR_14"
                    },
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
                "body":{
                    "error": {
                        "type": "invalid_request",
                        "message": "Payment method type not supported",
                        "code": "HE_03",
                        "reason": "debit mandate payment is not supported by paypal"
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
                "body":{
                    "error": {
                        "type": "invalid_request",
                        "message": "Payment method type not supported",
                        "code": "HE_03",
                        "reason": "debit mandate payment is not supported by paypal"
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
                "body":{
                    "error": {
                        "type": "invalid_request",
                        "message": "Payment method type not supported",
                        "code": "HE_03",
                        "reason": "debit mandate payment is not supported by paypal"
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
                "body":{
                    "error": {
                        "type": "invalid_request",
                        "message": "Payment method type not supported",
                        "code": "HE_03",
                        "reason": "debit mandate payment is not supported by paypal"
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
                "body":{
                    "error": {
                        "type": "invalid_request",
                        "message": "Payment method type not supported",
                        "code": "HE_03",
                        "reason": "debit mandate payment is not supported by paypal"
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
                "body":{
                    "error": {
                        "type": "invalid_request",
                        "message": "Payment method type not supported",
                        "code": "HE_03",
                        "reason": "debit mandate payment is not supported by paypal"
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
                },
            },
            "Response": {
                "status": 400,
                "body":{
                    "error": {
                        "type": "invalid_request",
                        "message": "Payment method type not supported",
                        "code": "HE_03",
                        "reason": "debit mandate payment is not supported by paypal"
                    }
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
                },
            },
            "Response": {
                "status": 400,
                "body":{
                    "error": {
                        "type": "invalid_request",
                        "message": "Payment method type not supported",
                        "code": "HE_03",
                        "reason": "debit mandate payment is not supported by paypal"
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
                        "message": "Setup Mandate flow for Paypal is not implemented",
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
                    "status": "processing"
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
                "status": 200,
                "body": {
                    "status": "requires_capture"
                }
            }
        },
    }
    };


