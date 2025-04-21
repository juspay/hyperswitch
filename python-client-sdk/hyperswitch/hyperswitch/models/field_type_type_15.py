from enum import Enum


class FieldTypeType15(str, Enum):
    USER_ADDRESS_CITY = "user_address_city"

    def __str__(self) -> str:
        return str(self.value)
