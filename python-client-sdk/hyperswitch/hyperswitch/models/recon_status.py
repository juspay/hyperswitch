from enum import Enum


class ReconStatus(str, Enum):
    ACTIVE = "active"
    DISABLED = "disabled"
    NOT_REQUESTED = "not_requested"
    REQUESTED = "requested"

    def __str__(self) -> str:
        return str(self.value)
