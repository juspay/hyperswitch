from enum import Enum


class RecurringDetailsType3Type(str, Enum):
    NETWORK_TRANSACTION_ID_AND_CARD_DETAILS = "network_transaction_id_and_card_details"

    def __str__(self) -> str:
        return str(self.value)
