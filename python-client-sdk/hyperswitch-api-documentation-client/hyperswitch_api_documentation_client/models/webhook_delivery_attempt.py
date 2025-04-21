from enum import Enum


class WebhookDeliveryAttempt(str, Enum):
    AUTOMATIC_RETRY = "automatic_retry"
    INITIAL_ATTEMPT = "initial_attempt"
    MANUAL_RETRY = "manual_retry"

    def __str__(self) -> str:
        return str(self.value)
