from enum import Enum


class ValueTypeType4Type(str, Enum):
    NUMBER_ARRAY = "number_array"

    def __str__(self) -> str:
        return str(self.value)
