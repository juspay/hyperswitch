from enum import Enum


class SingleType(str, Enum):
    SINGLE = "single"

    def __str__(self) -> str:
        return str(self.value)
