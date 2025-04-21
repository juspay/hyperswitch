from enum import Enum


class FieldTypeType35(str, Enum):
    USER_PIX_KEY = "user_pix_key"

    def __str__(self) -> str:
        return str(self.value)
