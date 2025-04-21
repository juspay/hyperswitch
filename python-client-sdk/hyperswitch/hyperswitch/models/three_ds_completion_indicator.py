from enum import Enum


class ThreeDsCompletionIndicator(str, Enum):
    N = "N"
    U = "U"
    Y = "Y"

    def __str__(self) -> str:
        return str(self.value)
