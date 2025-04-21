from enum import Enum


class SamsungPayCardBrand(str, Enum):
    AMEX = "amex"
    DISCOVER = "discover"
    MASTERCARD = "mastercard"
    UNKNOWN = "unknown"
    VISA = "visa"

    def __str__(self) -> str:
        return str(self.value)
