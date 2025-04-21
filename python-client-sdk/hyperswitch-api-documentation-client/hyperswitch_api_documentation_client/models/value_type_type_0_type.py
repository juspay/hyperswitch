from enum import Enum


class ValueTypeType0Type(str, Enum):
    NUMBER = "number"

    def __str__(self) -> str:
        return str(self.value)
