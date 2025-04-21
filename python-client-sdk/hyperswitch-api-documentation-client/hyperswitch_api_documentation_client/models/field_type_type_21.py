from enum import Enum


class FieldTypeType21(str, Enum):
    USER_SHIPPING_ADDRESS_LINE2 = "user_shipping_address_line2"

    def __str__(self) -> str:
        return str(self.value)
