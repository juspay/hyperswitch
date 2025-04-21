from enum import Enum


class FieldTypeType23(str, Enum):
    USER_SHIPPING_ADDRESS_PINCODE = "user_shipping_address_pincode"

    def __str__(self) -> str:
        return str(self.value)
