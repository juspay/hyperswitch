from enum import Enum


class PayoutStatus(str, Enum):
    CANCELLED = "cancelled"
    EXPIRED = "expired"
    FAILED = "failed"
    INELIGIBLE = "ineligible"
    INITIATED = "initiated"
    PENDING = "pending"
    REQUIRES_CONFIRMATION = "requires_confirmation"
    REQUIRES_CREATION = "requires_creation"
    REQUIRES_FULFILLMENT = "requires_fulfillment"
    REQUIRES_PAYOUT_METHOD_DATA = "requires_payout_method_data"
    REQUIRES_VENDOR_ACCOUNT_CREATION = "requires_vendor_account_creation"
    REVERSED = "reversed"
    SUCCESS = "success"

    def __str__(self) -> str:
        return str(self.value)
