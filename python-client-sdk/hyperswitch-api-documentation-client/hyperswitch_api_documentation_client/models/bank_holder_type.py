from enum import Enum


class BankHolderType(str, Enum):
    BUSINESS = "business"
    PERSONAL = "personal"

    def __str__(self) -> str:
        return str(self.value)
