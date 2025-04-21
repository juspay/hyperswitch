from enum import Enum


class VoucherDataType1(str, Enum):
    EFECTY = "efecty"

    def __str__(self) -> str:
        return str(self.value)
