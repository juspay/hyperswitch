from enum import Enum


class RoutingAlgorithmType2Type(str, Enum):
    VOLUME_SPLIT = "volume_split"

    def __str__(self) -> str:
        return str(self.value)
