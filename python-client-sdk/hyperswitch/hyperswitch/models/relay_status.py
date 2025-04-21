from enum import Enum


class RelayStatus(str, Enum):
    CREATED = "created"
    FAILURE = "failure"
    PENDING = "pending"
    SUCCESS = "success"

    def __str__(self) -> str:
        return str(self.value)
