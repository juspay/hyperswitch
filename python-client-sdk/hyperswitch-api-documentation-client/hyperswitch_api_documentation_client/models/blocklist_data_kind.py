from enum import Enum


class BlocklistDataKind(str, Enum):
    CARD_BIN = "card_bin"
    EXTENDED_CARD_BIN = "extended_card_bin"
    PAYMENT_METHOD = "payment_method"

    def __str__(self) -> str:
        return str(self.value)
