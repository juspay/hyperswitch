from enum import Enum


class FieldTypeType0(str, Enum):
    USER_CARD_NUMBER = "user_card_number"

    def __str__(self) -> str:
        return str(self.value)
