from enum import Enum


class SurchargeResponseType0Type(str, Enum):
    FIXED = "fixed"

    def __str__(self) -> str:
        return str(self.value)
