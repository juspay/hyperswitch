from enum import Enum


class FieldTypeType36(str, Enum):
    USER_CPF = "user_cpf"

    def __str__(self) -> str:
        return str(self.value)
