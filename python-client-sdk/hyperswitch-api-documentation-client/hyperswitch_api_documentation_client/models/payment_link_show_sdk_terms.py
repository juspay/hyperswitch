from enum import Enum


class PaymentLinkShowSdkTerms(str, Enum):
    ALWAYS = "always"
    AUTO = "auto"
    NEVER = "never"

    def __str__(self) -> str:
        return str(self.value)
