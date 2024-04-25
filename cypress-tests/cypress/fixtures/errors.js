export const refundErrors = {
    "paymentStatusProcessing": {
        "type": "invalid_request",
        "message": "This Payment could not be refund because it has a status of processing. The expected state is succeeded, partially_captured",
        "code": "IR_14"
    },
    "paymentStatusRequiresAction": {
        "type": "invalid_request",
        "message": "This Payment could not be refund because it has a status of requires_customer_action. The expected state is succeeded, partially_captured",
        "code": "IR_14"
    },
    "paymentStatusFailed": {
        "type": "invalid_request",
        "message": "This Payment could not be refund because it has a status of failed. The expected state is succeeded, partially_captured",
        "code": "IR_14"
    },
    "paymentStatusCancelled": {
        "type": "invalid_request",
        "message": "This Payment could not be refund because it has a status of cancelled. The expected state is succeeded, partially_captured",
        "code": "IR_14"
    },
    "paymentStatusRefunded": {
        "type": "invalid_request",
        "message": "This Payment could not be refund because it has a status of refunded. The expected state is succeeded, partially_captured",
        "code": "IR_14"
    },
    "paymentStatusRefundedPartially": {
        "type": "invalid_request",
        "message": "This Payment could not be refund because it has a status of partially_refunded. The expected state is succeeded, partially_captured",
        "code": "IR_14"
    },
    "paymentStatusCaptured": {
        "type": "invalid_request",
        "message": "This Payment could not be refund because it has a status of captured. The expected state is succeeded, partially_captured",
        "code": "IR_14"
    },
    "refundDoesNotExist": {
        "type": "invalid_request",
        "message": "Refund does not exist in our records.",
        "code": "HE_02"
    },
    "paymentStatusRequiresPaymentMethod": {
        "type": "invalid_request",
        "message": "This Payment could not be refund because it has a status of requires_payment_method. The expected state is succeeded, partially_captured",
        "code": "IR_14"
    }
};

export const captureErrors = {
    "paymentStatusRequiresPaymentMethod": {
        "type": "invalid_request",
        "message": "This Payment could not be captured because it has a payment.status of requires_payment_method. The expected state is requires_capture, partially_captured_and_capturable, processing",
        "code": "IR_14"
        },
};

// Connector specific
export const paymentMethodErrors = {
    "trustpay" : {
        "paymentMethodUnsupportedError": {
            "type": "invalid_request",
            "message": "Payment method type not supported",
            "code": "HE_03",
            "reason": "manual is not supported by trustpay"
        }
    },
};

// Connector agnostic
export const paymentMethodCAErrors = {
   "paymentMethodDoesNotExist": {
        "type": "invalid_request",
        "message": "Payment method does not exist in our records",
        "code": "HE_02"
    },
    "tokenOrMethodDataMissing": {
        "type": "invalid_request",
        "message": "A payment token or payment method data is required",
        "code": "IR_06"
    },
};

export const paymentErrors = {
    "paymentDoesNotExist": {
        "type": "invalid_request",
        "message": "Payment does not exist in our records",
        "code": "HE_02"
    }
}

