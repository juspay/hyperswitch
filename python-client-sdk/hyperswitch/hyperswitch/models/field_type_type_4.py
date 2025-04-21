from enum import Enum


class FieldTypeType4(str, Enum):
    USER_CARD_NETWORK = "user_card_network"

    def __str__(self) -> str:
        return str(self.value)
