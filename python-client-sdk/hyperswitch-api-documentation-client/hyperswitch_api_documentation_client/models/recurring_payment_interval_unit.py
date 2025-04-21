from enum import Enum


class RecurringPaymentIntervalUnit(str, Enum):
    DAY = "day"
    HOUR = "hour"
    MINUTE = "minute"
    MONTH = "month"
    YEAR = "year"

    def __str__(self) -> str:
        return str(self.value)
