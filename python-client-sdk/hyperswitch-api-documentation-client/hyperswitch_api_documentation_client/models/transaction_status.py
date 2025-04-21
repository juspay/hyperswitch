from enum import Enum


class TransactionStatus(str, Enum):
    A = "A"
    C = "C"
    D = "D"
    I = "I"
    N = "N"
    R = "R"
    U = "U"
    Y = "Y"

    def __str__(self) -> str:
        return str(self.value)
