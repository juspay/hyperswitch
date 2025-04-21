from enum import Enum


class BankType(str, Enum):
    CHECKING = "checking"
    SAVINGS = "savings"

    def __str__(self) -> str:
        return str(self.value)
