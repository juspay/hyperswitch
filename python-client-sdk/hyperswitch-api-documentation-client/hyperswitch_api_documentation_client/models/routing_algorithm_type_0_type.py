from enum import Enum


class RoutingAlgorithmType0Type(str, Enum):
    SINGLE = "single"

    def __str__(self) -> str:
        return str(self.value)
