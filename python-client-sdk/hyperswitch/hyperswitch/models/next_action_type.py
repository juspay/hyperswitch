from enum import Enum


class NextActionType(str, Enum):
    COLLECT_OTP = "collect_otp"
    DISPLAY_BANK_TRANSFER_INFORMATION = "display_bank_transfer_information"
    DISPLAY_QR_CODE = "display_qr_code"
    DISPLAY_WAIT_SCREEN = "display_wait_screen"
    INVOKE_SDK_CLIENT = "invoke_sdk_client"
    REDIRECT_TO_URL = "redirect_to_url"
    TRIGGER_API = "trigger_api"

    def __str__(self) -> str:
        return str(self.value)
