from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.applepay_payment_method import ApplepayPaymentMethod


T = TypeVar("T", bound="ApplePayWalletData")


@_attrs_define
class ApplePayWalletData:
    """
    Attributes:
        payment_data (str): The payment data of Apple pay
        payment_method (ApplepayPaymentMethod):
        transaction_identifier (str): The unique identifier for the transaction
    """

    payment_data: str
    payment_method: "ApplepayPaymentMethod"
    transaction_identifier: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payment_data = self.payment_data

        payment_method = self.payment_method.to_dict()

        transaction_identifier = self.transaction_identifier

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_data": payment_data,
                "payment_method": payment_method,
                "transaction_identifier": transaction_identifier,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.applepay_payment_method import ApplepayPaymentMethod

        d = dict(src_dict)
        payment_data = d.pop("payment_data")

        payment_method = ApplepayPaymentMethod.from_dict(d.pop("payment_method"))

        transaction_identifier = d.pop("transaction_identifier")

        apple_pay_wallet_data = cls(
            payment_data=payment_data,
            payment_method=payment_method,
            transaction_identifier=transaction_identifier,
        )

        apple_pay_wallet_data.additional_properties = d
        return apple_pay_wallet_data

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
