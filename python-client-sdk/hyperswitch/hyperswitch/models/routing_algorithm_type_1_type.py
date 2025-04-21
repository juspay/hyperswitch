from enum import Enum


class RoutingAlgorithmType1Type(str, Enum):
    PRIORITY = "priority"

    def __str__(self) -> str:
        return str(self.value)
