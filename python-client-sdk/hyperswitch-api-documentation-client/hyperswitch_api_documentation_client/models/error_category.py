from enum import Enum


class ErrorCategory(str, Enum):
    FRM_DECLINE = "frm_decline"
    ISSUE_WITH_PAYMENT_METHOD = "issue_with_payment_method"
    PROCESSOR_DECLINE_INCORRECT_DATA = "processor_decline_incorrect_data"
    PROCESSOR_DECLINE_UNAUTHORIZED = "processor_decline_unauthorized"
    PROCESSOR_DOWNTIME = "processor_downtime"

    def __str__(self) -> str:
        return str(self.value)
