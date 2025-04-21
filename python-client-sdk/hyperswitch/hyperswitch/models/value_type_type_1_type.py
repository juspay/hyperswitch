from enum import Enum


class ValueTypeType1Type(str, Enum):
    ENUM_VARIANT = "enum_variant"

    def __str__(self) -> str:
        return str(self.value)
