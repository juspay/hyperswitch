from enum import Enum


class FieldTypeType22(str, Enum):
    USER_SHIPPING_ADDRESS_CITY = "user_shipping_address_city"

    def __str__(self) -> str:
        return str(self.value)
