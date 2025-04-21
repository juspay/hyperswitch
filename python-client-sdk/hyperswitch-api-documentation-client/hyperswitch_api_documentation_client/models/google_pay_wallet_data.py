from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.google_pay_payment_method_info import GooglePayPaymentMethodInfo
    from ..models.gpay_tokenization_data import GpayTokenizationData


T = TypeVar("T", bound="GooglePayWalletData")


@_attrs_define
class GooglePayWalletData:
    """
    Attributes:
        type_ (str): The type of payment method
        description (str): User-facing message to describe the payment method that funds this transaction.
        info (GooglePayPaymentMethodInfo):
        tokenization_data (GpayTokenizationData):
    """

    type_: str
    description: str
    info: "GooglePayPaymentMethodInfo"
    tokenization_data: "GpayTokenizationData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_

        description = self.description

        info = self.info.to_dict()

        tokenization_data = self.tokenization_data.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type_,
                "description": description,
                "info": info,
                "tokenization_data": tokenization_data,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.google_pay_payment_method_info import GooglePayPaymentMethodInfo
        from ..models.gpay_tokenization_data import GpayTokenizationData

        d = dict(src_dict)
        type_ = d.pop("type")

        description = d.pop("description")

        info = GooglePayPaymentMethodInfo.from_dict(d.pop("info"))

        tokenization_data = GpayTokenizationData.from_dict(d.pop("tokenization_data"))

        google_pay_wallet_data = cls(
            type_=type_,
            description=description,
            info=info,
            tokenization_data=tokenization_data,
        )

        google_pay_wallet_data.additional_properties = d
        return google_pay_wallet_data

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
