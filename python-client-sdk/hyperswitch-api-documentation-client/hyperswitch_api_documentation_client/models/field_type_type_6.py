from enum import Enum


class FieldTypeType6(str, Enum):
    USER_EMAIL_ADDRESS = "user_email_address"

    def __str__(self) -> str:
        return str(self.value)
