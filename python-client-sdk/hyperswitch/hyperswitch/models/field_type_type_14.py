from enum import Enum


class FieldTypeType14(str, Enum):
    USER_ADDRESS_LINE2 = "user_address_line2"

    def __str__(self) -> str:
        return str(self.value)
