from enum import Enum


class FieldTypeType13(str, Enum):
    USER_ADDRESS_LINE1 = "user_address_line1"

    def __str__(self) -> str:
        return str(self.value)
