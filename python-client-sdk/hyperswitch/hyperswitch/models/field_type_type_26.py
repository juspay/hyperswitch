from enum import Enum


class FieldTypeType26(str, Enum):
    USER_SOCIAL_SECURITY_NUMBER = "user_social_security_number"

    def __str__(self) -> str:
        return str(self.value)
