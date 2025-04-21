from enum import Enum


class AuthenticationType(str, Enum):
    NO_THREE_DS = "no_three_ds"
    THREE_DS = "three_ds"

    def __str__(self) -> str:
        return str(self.value)
