const successfulNo3DSCardDetails = {
    "card_number": "4242424242424242",
    "card_exp_month": "01",
    "card_exp_year": "25",
    "card_holder_name": "joseph Doe",
    "card_cvc": "123"
};

const successfulThreeDSTestCardDetails = {
    "card_number": "4000000000001091",
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
                    "status": "succeeded"
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
                    "status": "succeeded",
                    "amount": 6500,
                    "amount_capturable": 0,
                    "amount_received": 6500,
    
                }
            }
        },
        "PartialCapture": {
            "Request": {
             
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
                    "status": "pending",
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
                    "status": "pending",
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
                    "status": "pending",
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
                "status": 200,
                "body": {
                    "status": "succeeded"
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
                "status": 200,
                "body": {
                    "status": "requires_customer_action"
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
                "status": 200,
                "body": {
                    "status": "succeeded"
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
                "status": 200,
                "body": {
                    "status": "requires_capture"
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
                "status": 200,
                "body": {
                    "status": "succeeded"
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
                "status": 200,
                "body": {
                    "status": "requires_capture"
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
                "status": 200,
                "body": {
                    "status": "requires_capture"
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
                "status": 200,
                "body": {
                    "status": "requires_capture"
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
                "status": 200,
                "body": {
                    "status": "succeeded"
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
                "status": 200,
                "body": {
                    "status": "requires_capture"
                }
            }
        },
    }
    };