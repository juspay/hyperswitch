from enum import Enum


class FieldTypeType37(str, Enum):
    USER_CNPJ = "user_cnpj"

    def __str__(self) -> str:
        return str(self.value)
