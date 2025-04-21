from enum import Enum


class ConnectorSelectionType1Type(str, Enum):
    VOLUME_SPLIT = "volume_split"

    def __str__(self) -> str:
        return str(self.value)
