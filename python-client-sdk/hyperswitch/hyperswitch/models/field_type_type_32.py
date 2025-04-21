from enum import Enum


class FieldTypeType32(str, Enum):
    USER_DATE_OF_BIRTH = "user_date_of_birth"

    def __str__(self) -> str:
        return str(self.value)
