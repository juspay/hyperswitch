from enum import Enum


class IntentStatus(str, Enum):
    CANCELLED = "cancelled"
    FAILED = "failed"
    PARTIALLY_CAPTURED = "partially_captured"
    PARTIALLY_CAPTURED_AND_CAPTURABLE = "partially_captured_and_capturable"
    PROCESSING = "processing"
    REQUIRES_CAPTURE = "requires_capture"
    REQUIRES_CONFIRMATION = "requires_confirmation"
    REQUIRES_CUSTOMER_ACTION = "requires_customer_action"
    REQUIRES_MERCHANT_ACTION = "requires_merchant_action"
    REQUIRES_PAYMENT_METHOD = "requires_payment_method"
    SUCCEEDED = "succeeded"

    def __str__(self) -> str:
        return str(self.value)
