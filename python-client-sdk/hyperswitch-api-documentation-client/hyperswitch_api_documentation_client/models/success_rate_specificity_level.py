from enum import Enum


class SuccessRateSpecificityLevel(str, Enum):
    GLOBAL = "global"
    MERCHANT = "merchant"

    def __str__(self) -> str:
        return str(self.value)
