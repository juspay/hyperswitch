from enum import Enum


class PaymentExperience(str, Enum):
    COLLECT_OTP = "collect_otp"
    DISPLAY_QR_CODE = "display_qr_code"
    DISPLAY_WAIT_SCREEN = "display_wait_screen"
    INVOKE_PAYMENT_APP = "invoke_payment_app"
    INVOKE_SDK_CLIENT = "invoke_sdk_client"
    LINK_WALLET = "link_wallet"
    ONE_CLICK = "one_click"
    REDIRECT_TO_URL = "redirect_to_url"

    def __str__(self) -> str:
        return str(self.value)
