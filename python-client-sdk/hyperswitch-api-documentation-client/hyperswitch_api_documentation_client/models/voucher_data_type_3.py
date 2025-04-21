from enum import Enum


class VoucherDataType3(str, Enum):
    RED_COMPRA = "red_compra"

    def __str__(self) -> str:
        return str(self.value)
