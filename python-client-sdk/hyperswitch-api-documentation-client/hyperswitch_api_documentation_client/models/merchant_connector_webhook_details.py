from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define

T = TypeVar("T", bound="MerchantConnectorWebhookDetails")


@_attrs_define
class MerchantConnectorWebhookDetails:
    """
    Attributes:
        merchant_secret (str):  Example: 12345678900987654321.
        additional_secret (str):  Example: 12345678900987654321.
    """

    merchant_secret: str
    additional_secret: str

    def to_dict(self) -> dict[str, Any]:
        merchant_secret = self.merchant_secret

        additional_secret = self.additional_secret

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "merchant_secret": merchant_secret,
                "additional_secret": additional_secret,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        merchant_secret = d.pop("merchant_secret")

        additional_secret = d.pop("additional_secret")

        merchant_connector_webhook_details = cls(
            merchant_secret=merchant_secret,
            additional_secret=additional_secret,
        )

        return merchant_connector_webhook_details
