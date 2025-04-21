from enum import Enum


class ValueTypeType2Type(str, Enum):
    METADATA_VARIANT = "metadata_variant"

    def __str__(self) -> str:
        return str(self.value)
