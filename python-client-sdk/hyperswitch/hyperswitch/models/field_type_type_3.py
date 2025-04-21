from enum import Enum


class FieldTypeType3(str, Enum):
    USER_CARD_CVC = "user_card_cvc"

    def __str__(self) -> str:
        return str(self.value)
