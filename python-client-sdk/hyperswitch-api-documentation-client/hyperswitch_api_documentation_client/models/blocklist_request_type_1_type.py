from enum import Enum


class BlocklistRequestType1Type(str, Enum):
    FINGERPRINT = "fingerprint"

    def __str__(self) -> str:
        return str(self.value)
