from enum import Enum


class SizeVariants(str, Enum):
    CONTAIN = "contain"
    COVER = "cover"

    def __str__(self) -> str:
        return str(self.value)
