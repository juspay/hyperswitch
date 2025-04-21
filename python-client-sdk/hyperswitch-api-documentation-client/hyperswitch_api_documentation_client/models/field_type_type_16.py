from enum import Enum


class FieldTypeType16(str, Enum):
    USER_ADDRESS_PINCODE = "user_address_pincode"

    def __str__(self) -> str:
        return str(self.value)
