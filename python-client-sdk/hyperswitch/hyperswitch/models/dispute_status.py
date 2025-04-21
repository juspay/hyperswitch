from enum import Enum


class DisputeStatus(str, Enum):
    DISPUTE_ACCEPTED = "dispute_accepted"
    DISPUTE_CANCELLED = "dispute_cancelled"
    DISPUTE_CHALLENGED = "dispute_challenged"
    DISPUTE_EXPIRED = "dispute_expired"
    DISPUTE_LOST = "dispute_lost"
    DISPUTE_OPENED = "dispute_opened"
    DISPUTE_WON = "dispute_won"

    def __str__(self) -> str:
        return str(self.value)
