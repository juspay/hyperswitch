from enum import Enum


class RoutingAlgorithmKind(str, Enum):
    ADVANCED = "advanced"
    DYNAMIC = "dynamic"
    PRIORITY = "priority"
    SINGLE = "single"
    VOLUME_SPLIT = "volume_split"

    def __str__(self) -> str:
        return str(self.value)
