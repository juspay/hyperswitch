from enum import Enum


class FrmAction(str, Enum):
    AUTO_REFUND = "auto_refund"
    CANCEL_TXN = "cancel_txn"
    MANUAL_REVIEW = "manual_review"

    def __str__(self) -> str:
        return str(self.value)
