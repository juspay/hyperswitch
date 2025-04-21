from enum import Enum


class ValueTypeType6Type(str, Enum):
    NUMBER_COMPARISON_ARRAY = "number_comparison_array"

    def __str__(self) -> str:
        return str(self.value)
