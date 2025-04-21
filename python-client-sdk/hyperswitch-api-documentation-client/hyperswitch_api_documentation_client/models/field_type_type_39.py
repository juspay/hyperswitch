from enum import Enum


class FieldTypeType39(str, Enum):
    USER_BSB_NUMBER = "user_bsb_number"

    def __str__(self) -> str:
        return str(self.value)
