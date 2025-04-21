from enum import Enum


class ValueTypeType5Type(str, Enum):
    ENUM_VARIANT_ARRAY = "enum_variant_array"

    def __str__(self) -> str:
        return str(self.value)
