from enum import Enum


class DisputeStage(str, Enum):
    DISPUTE = "dispute"
    PRE_ARBITRATION = "pre_arbitration"
    PRE_DISPUTE = "pre_dispute"

    def __str__(self) -> str:
        return str(self.value)
