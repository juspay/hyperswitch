from enum import Enum


class FieldTypeType28(str, Enum):
    USER_BANK = "user_bank"

    def __str__(self) -> str:
        return str(self.value)
