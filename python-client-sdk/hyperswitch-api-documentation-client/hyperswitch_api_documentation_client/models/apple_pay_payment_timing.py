from enum import Enum


class ApplePayPaymentTiming(str, Enum):
    IMMEDIATE = "immediate"
    RECURRING = "recurring"

    def __str__(self) -> str:
        return str(self.value)
