from enum import Enum


class ConnectorSelectionType0Type(str, Enum):
    PRIORITY = "priority"

    def __str__(self) -> str:
        return str(self.value)
