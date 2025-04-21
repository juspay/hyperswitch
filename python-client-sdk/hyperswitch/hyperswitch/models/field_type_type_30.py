from enum import Enum


class FieldTypeType30(str, Enum):
    TEXT = "text"

    def __str__(self) -> str:
        return str(self.value)
