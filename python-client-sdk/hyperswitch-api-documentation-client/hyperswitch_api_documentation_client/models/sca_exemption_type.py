from enum import Enum


class ScaExemptionType(str, Enum):
    LOW_VALUE = "low_value"
    TRANSACTION_RISK_ANALYSIS = "transaction_risk_analysis"

    def __str__(self) -> str:
        return str(self.value)
