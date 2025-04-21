from enum import Enum


class FieldTypeType41(str, Enum):
    USER_BANK_ROUTING_NUMBER = "user_bank_routing_number"

    def __str__(self) -> str:
        return str(self.value)
