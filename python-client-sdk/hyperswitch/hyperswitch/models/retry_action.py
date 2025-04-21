from enum import Enum


class RetryAction(str, Enum):
    MANUAL_RETRY = "manual_retry"
    REQUEUE = "requeue"

    def __str__(self) -> str:
        return str(self.value)
