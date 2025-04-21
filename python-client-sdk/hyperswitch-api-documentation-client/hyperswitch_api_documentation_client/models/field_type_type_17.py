from enum import Enum


class FieldTypeType17(str, Enum):
    USER_ADDRESS_STATE = "user_address_state"

    def __str__(self) -> str:
        return str(self.value)
