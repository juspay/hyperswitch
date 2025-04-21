from enum import Enum


class PaymentMethodIssuerCode(str, Enum):
    JP_APPLEPAY = "jp_applepay"
    JP_BACS = "jp_bacs"
    JP_GIROPAY = "jp_giropay"
    JP_GOOGLEPAY = "jp_googlepay"
    JP_HDFC = "jp_hdfc"
    JP_ICICI = "jp_icici"
    JP_PHONEPAY = "jp_phonepay"
    JP_SEPA = "jp_sepa"
    JP_SOFORT = "jp_sofort"
    JP_WECHAT = "jp_wechat"

    def __str__(self) -> str:
        return str(self.value)
