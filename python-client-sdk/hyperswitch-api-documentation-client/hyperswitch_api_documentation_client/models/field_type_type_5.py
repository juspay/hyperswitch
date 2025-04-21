from enum import Enum


class FieldTypeType5(str, Enum):
    USER_FULL_NAME = "user_full_name"

    def __str__(self) -> str:
        return str(self.value)
