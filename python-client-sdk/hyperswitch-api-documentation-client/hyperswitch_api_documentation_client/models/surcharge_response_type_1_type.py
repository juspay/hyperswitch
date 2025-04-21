from enum import Enum


class SurchargeResponseType1Type(str, Enum):
    RATE = "rate"

    def __str__(self) -> str:
        return str(self.value)
