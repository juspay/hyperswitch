from enum import Enum


class ValueTypeType3Type(str, Enum):
    STR_VALUE = "str_value"

    def __str__(self) -> str:
        return str(self.value)
