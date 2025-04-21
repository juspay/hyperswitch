from enum import Enum


class PaymentMethodStatus(str, Enum):
    ACTIVE = "active"
    AWAITING_DATA = "awaiting_data"
    INACTIVE = "inactive"
    PROCESSING = "processing"

    def __str__(self) -> str:
        return str(self.value)
