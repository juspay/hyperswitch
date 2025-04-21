from enum import Enum


class VoucherDataType7(str, Enum):
    OXXO = "oxxo"

    def __str__(self) -> str:
        return str(self.value)
