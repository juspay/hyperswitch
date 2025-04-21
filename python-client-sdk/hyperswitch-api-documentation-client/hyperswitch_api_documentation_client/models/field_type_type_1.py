from enum import Enum


class FieldTypeType1(str, Enum):
    USER_CARD_EXPIRY_MONTH = "user_card_expiry_month"

    def __str__(self) -> str:
        return str(self.value)
