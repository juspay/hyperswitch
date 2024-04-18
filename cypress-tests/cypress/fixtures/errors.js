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
    }
};
