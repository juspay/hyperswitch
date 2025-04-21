from enum import Enum


class RefundType(str, Enum):
    INSTANT = "instant"
    SCHEDULED = "scheduled"

    def __str__(self) -> str:
        return str(self.value)
