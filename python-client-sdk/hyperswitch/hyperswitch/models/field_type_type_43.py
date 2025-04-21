from enum import Enum


class FieldTypeType43(str, Enum):
    USER_CLIENT_IDENTIFIER = "user_client_identifier"

    def __str__(self) -> str:
        return str(self.value)
