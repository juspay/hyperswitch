from enum import Enum


class SamsungPayAmountFormat(str, Enum):
    FORMAT_TOTAL_ESTIMATED_AMOUNT = "FORMAT_TOTAL_ESTIMATED_AMOUNT"
    FORMAT_TOTAL_PRICE_ONLY = "FORMAT_TOTAL_PRICE_ONLY"

    def __str__(self) -> str:
        return str(self.value)
