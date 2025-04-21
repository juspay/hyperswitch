from enum import Enum


class SamsungPayProtocolType(str, Enum):
    PROTOCOL3DS = "PROTOCOL3DS"

    def __str__(self) -> str:
        return str(self.value)
