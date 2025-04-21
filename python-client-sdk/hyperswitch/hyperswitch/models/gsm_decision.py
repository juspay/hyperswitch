from enum import Enum


class GsmDecision(str, Enum):
    DO_DEFAULT = "do_default"
    REQUEUE = "requeue"
    RETRY = "retry"

    def __str__(self) -> str:
        return str(self.value)
