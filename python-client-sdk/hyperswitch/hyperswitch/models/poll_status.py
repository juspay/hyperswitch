from enum import Enum


class PollStatus(str, Enum):
    COMPLETED = "completed"
    NOT_FOUND = "not_found"
    PENDING = "pending"

    def __str__(self) -> str:
        return str(self.value)
