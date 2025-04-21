from enum import Enum


class FieldTypeType19(str, Enum):
    USER_SHIPPING_NAME = "user_shipping_name"

    def __str__(self) -> str:
        return str(self.value)
