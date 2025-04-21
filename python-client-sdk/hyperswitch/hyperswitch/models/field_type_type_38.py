from enum import Enum


class FieldTypeType38(str, Enum):
    USER_IBAN = "user_iban"

    def __str__(self) -> str:
        return str(self.value)
