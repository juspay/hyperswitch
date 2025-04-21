from enum import Enum


class FieldTypeType33(str, Enum):
    USER_VPA_ID = "user_vpa_id"

    def __str__(self) -> str:
        return str(self.value)
