from enum import Enum


class RelayType(str, Enum):
    REFUND = "refund"

    def __str__(self) -> str:
        return str(self.value)
