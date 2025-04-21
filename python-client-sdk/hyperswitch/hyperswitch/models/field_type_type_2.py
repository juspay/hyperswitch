from enum import Enum


class FieldTypeType2(str, Enum):
    USER_CARD_EXPIRY_YEAR = "user_card_expiry_year"

    def __str__(self) -> str:
        return str(self.value)
