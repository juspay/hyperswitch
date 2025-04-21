from enum import Enum


class EventClass(str, Enum):
    DISPUTES = "disputes"
    MANDATES = "mandates"
    PAYMENTS = "payments"
    PAYOUTS = "payouts"
    REFUNDS = "refunds"

    def __str__(self) -> str:
        return str(self.value)
