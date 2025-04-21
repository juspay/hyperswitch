from enum import Enum


class MobilePaymentConsent(str, Enum):
    CONSENT_NOT_REQUIRED = "consent_not_required"
    CONSENT_OPTIONAL = "consent_optional"
    CONSENT_REQUIRED = "consent_required"

    def __str__(self) -> str:
        return str(self.value)
