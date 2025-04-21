from enum import Enum


class FieldTypeType27(str, Enum):
    USER_BLIK_CODE = "user_blik_code"

    def __str__(self) -> str:
        return str(self.value)
