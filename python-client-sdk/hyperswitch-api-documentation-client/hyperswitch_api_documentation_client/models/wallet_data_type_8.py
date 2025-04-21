from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.apple_pay_wallet_data import ApplePayWalletData


T = TypeVar("T", bound="WalletDataType8")


@_attrs_define
class WalletDataType8:
    """
    Attributes:
        apple_pay (ApplePayWalletData):
    """

    apple_pay: "ApplePayWalletData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        apple_pay = self.apple_pay.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "apple_pay": apple_pay,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.apple_pay_wallet_data import ApplePayWalletData

        d = dict(src_dict)
        apple_pay = ApplePayWalletData.from_dict(d.pop("apple_pay"))

        wallet_data_type_8 = cls(
            apple_pay=apple_pay,
        )

        wallet_data_type_8.additional_properties = d
        return wallet_data_type_8

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
