from enum import Enum


class PayoutSendPriority(str, Enum):
    CROSS_BORDER = "cross_border"
    FAST = "fast"
    INSTANT = "instant"
    INTERNAL = "internal"
    REGULAR = "regular"
    WIRE = "wire"

    def __str__(self) -> str:
        return str(self.value)
