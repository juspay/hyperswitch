from enum import Enum


class FieldTypeType40(str, Enum):
    USER_BANK_SORT_CODE = "user_bank_sort_code"

    def __str__(self) -> str:
        return str(self.value)
