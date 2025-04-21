from enum import Enum


class FieldTypeType12(str, Enum):
    USER_BILLING_NAME = "user_billing_name"

    def __str__(self) -> str:
        return str(self.value)
