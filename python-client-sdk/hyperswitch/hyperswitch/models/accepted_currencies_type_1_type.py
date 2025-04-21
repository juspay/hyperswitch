from enum import Enum


class AcceptedCurrenciesType1Type(str, Enum):
    DISABLE_ONLY = "disable_only"

    def __str__(self) -> str:
        return str(self.value)
