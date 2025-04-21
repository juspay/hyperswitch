from enum import Enum


class VoucherDataType2(str, Enum):
    PAGO_EFECTIVO = "pago_efectivo"

    def __str__(self) -> str:
        return str(self.value)
