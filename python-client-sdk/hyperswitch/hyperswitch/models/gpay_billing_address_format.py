from enum import Enum


class GpayBillingAddressFormat(str, Enum):
    FULL = "FULL"
    MIN = "MIN"

    def __str__(self) -> str:
        return str(self.value)
