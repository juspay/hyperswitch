from enum import Enum


class FieldTypeType8(str, Enum):
    USER_PHONE_NUMBER_COUNTRY_CODE = "user_phone_number_country_code"

    def __str__(self) -> str:
        return str(self.value)
