from enum import Enum


class RecurringDetailsType1Type(str, Enum):
    PAYMENT_METHOD_ID = "payment_method_id"

    def __str__(self) -> str:
        return str(self.value)
