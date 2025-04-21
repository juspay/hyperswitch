from enum import Enum


class FieldTypeType29(str, Enum):
    USER_BANK_ACCOUNT_NUMBER = "user_bank_account_number"

    def __str__(self) -> str:
        return str(self.value)
