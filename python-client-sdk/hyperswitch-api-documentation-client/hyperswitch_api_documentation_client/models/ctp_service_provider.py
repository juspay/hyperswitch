from enum import Enum


class CtpServiceProvider(str, Enum):
    MASTERCARD = "mastercard"
    VISA = "visa"

    def __str__(self) -> str:
        return str(self.value)
