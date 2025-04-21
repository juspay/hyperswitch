from enum import Enum


class RefundStatus(str, Enum):
    FAILED = "failed"
    PENDING = "pending"
    REVIEW = "review"
    SUCCEEDED = "succeeded"

    def __str__(self) -> str:
        return str(self.value)
