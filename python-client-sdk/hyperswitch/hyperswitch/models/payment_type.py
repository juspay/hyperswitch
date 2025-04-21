from enum import Enum


class PaymentType(str, Enum):
    NEW_MANDATE = "new_mandate"
    NORMAL = "normal"
    RECURRING_MANDATE = "recurring_mandate"
    SETUP_MANDATE = "setup_mandate"

    def __str__(self) -> str:
        return str(self.value)
