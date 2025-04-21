from enum import Enum


class RoutingAlgorithmType3Type(str, Enum):
    ADVANCED = "advanced"

    def __str__(self) -> str:
        return str(self.value)
