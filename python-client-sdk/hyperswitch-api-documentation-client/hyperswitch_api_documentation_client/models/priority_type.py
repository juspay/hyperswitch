from enum import Enum


class PriorityType(str, Enum):
    PRIORITY = "priority"

    def __str__(self) -> str:
        return str(self.value)
