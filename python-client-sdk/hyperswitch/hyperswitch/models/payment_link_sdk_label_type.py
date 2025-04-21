from enum import Enum


class PaymentLinkSdkLabelType(str, Enum):
    ABOVE = "above"
    FLOATING = "floating"
    NEVER = "never"

    def __str__(self) -> str:
        return str(self.value)
