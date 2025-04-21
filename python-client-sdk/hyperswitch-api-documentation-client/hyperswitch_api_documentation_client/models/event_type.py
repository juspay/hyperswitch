from enum import Enum


class EventType(str, Enum):
    ACTION_REQUIRED = "action_required"
    DISPUTE_ACCEPTED = "dispute_accepted"
    DISPUTE_CANCELLED = "dispute_cancelled"
    DISPUTE_CHALLENGED = "dispute_challenged"
    DISPUTE_EXPIRED = "dispute_expired"
    DISPUTE_LOST = "dispute_lost"
    DISPUTE_OPENED = "dispute_opened"
    DISPUTE_WON = "dispute_won"
    MANDATE_ACTIVE = "mandate_active"
    MANDATE_REVOKED = "mandate_revoked"
    PAYMENT_AUTHORIZED = "payment_authorized"
    PAYMENT_CANCELLED = "payment_cancelled"
    PAYMENT_CAPTURED = "payment_captured"
    PAYMENT_FAILED = "payment_failed"
    PAYMENT_PROCESSING = "payment_processing"
    PAYMENT_SUCCEEDED = "payment_succeeded"
    PAYOUT_CANCELLED = "payout_cancelled"
    PAYOUT_EXPIRED = "payout_expired"
    PAYOUT_FAILED = "payout_failed"
    PAYOUT_INITIATED = "payout_initiated"
    PAYOUT_PROCESSING = "payout_processing"
    PAYOUT_REVERSED = "payout_reversed"
    PAYOUT_SUCCESS = "payout_success"
    REFUND_FAILED = "refund_failed"
    REFUND_SUCCEEDED = "refund_succeeded"

    def __str__(self) -> str:
        return str(self.value)
