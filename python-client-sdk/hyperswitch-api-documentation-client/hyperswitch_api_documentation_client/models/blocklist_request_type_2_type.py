from enum import Enum


class BlocklistRequestType2Type(str, Enum):
    EXTENDED_CARD_BIN = "extended_card_bin"

    def __str__(self) -> str:
        return str(self.value)
