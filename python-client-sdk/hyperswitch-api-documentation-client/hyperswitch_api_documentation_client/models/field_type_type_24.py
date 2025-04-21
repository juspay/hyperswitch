from enum import Enum


class FieldTypeType24(str, Enum):
    USER_SHIPPING_ADDRESS_STATE = "user_shipping_address_state"

    def __str__(self) -> str:
        return str(self.value)
