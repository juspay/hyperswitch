from enum import Enum


class BlocklistRequestType0Type(str, Enum):
    CARD_BIN = "card_bin"

    def __str__(self) -> str:
        return str(self.value)
