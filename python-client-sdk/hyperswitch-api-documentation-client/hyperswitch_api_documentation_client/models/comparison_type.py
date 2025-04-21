from enum import Enum


class ComparisonType(str, Enum):
    EQUAL = "equal"
    GREATER_THAN = "greater_than"
    GREATER_THAN_EQUAL = "greater_than_equal"
    LESS_THAN = "less_than"
    LESS_THAN_EQUAL = "less_than_equal"
    NOT_EQUAL = "not_equal"

    def __str__(self) -> str:
        return str(self.value)
