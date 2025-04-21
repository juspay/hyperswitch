from enum import Enum


class PayoutType(str, Enum):
    BANK = "bank"
    CARD = "card"
    WALLET = "wallet"

    def __str__(self) -> str:
        return str(self.value)
