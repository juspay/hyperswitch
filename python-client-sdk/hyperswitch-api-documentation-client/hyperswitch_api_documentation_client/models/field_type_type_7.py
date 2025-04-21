from enum import Enum


class FieldTypeType7(str, Enum):
    USER_PHONE_NUMBER = "user_phone_number"

    def __str__(self) -> str:
        return str(self.value)
