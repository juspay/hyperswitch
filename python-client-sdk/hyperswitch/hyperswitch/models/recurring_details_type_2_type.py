from enum import Enum


class RecurringDetailsType2Type(str, Enum):
    PROCESSOR_PAYMENT_TOKEN = "processor_payment_token"

    def __str__(self) -> str:
        return str(self.value)
