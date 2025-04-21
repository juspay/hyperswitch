from enum import Enum


class FieldTypeType42(str, Enum):
    USER_MSISDN = "user_msisdn"

    def __str__(self) -> str:
        return str(self.value)
