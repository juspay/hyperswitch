from enum import Enum


class TransactionType(str, Enum):
    PAYMENT = "payment"
    PAYOUT = "payout"

    def __str__(self) -> str:
        return str(self.value)
