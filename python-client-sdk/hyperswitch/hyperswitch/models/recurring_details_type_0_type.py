from enum import Enum


class RecurringDetailsType0Type(str, Enum):
    MANDATE_ID = "mandate_id"

    def __str__(self) -> str:
        return str(self.value)
