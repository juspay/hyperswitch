from enum import Enum


class VoucherDataType4(str, Enum):
    RED_PAGOS = "red_pagos"

    def __str__(self) -> str:
        return str(self.value)
