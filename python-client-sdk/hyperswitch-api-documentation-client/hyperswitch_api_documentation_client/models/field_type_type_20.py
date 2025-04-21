from enum import Enum


class FieldTypeType20(str, Enum):
    USER_SHIPPING_ADDRESS_LINE1 = "user_shipping_address_line1"

    def __str__(self) -> str:
        return str(self.value)
