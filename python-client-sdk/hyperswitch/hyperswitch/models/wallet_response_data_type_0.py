from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.wallet_additional_data_for_card import WalletAdditionalDataForCard


T = TypeVar("T", bound="WalletResponseDataType0")


@_attrs_define
class WalletResponseDataType0:
    """
    Attributes:
        apple_pay (WalletAdditionalDataForCard):
    """

    apple_pay: "WalletAdditionalDataForCard"
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
        from ..models.wallet_additional_data_for_card import WalletAdditionalDataForCard

        d = dict(src_dict)
        apple_pay = WalletAdditionalDataForCard.from_dict(d.pop("apple_pay"))

        wallet_response_data_type_0 = cls(
            apple_pay=apple_pay,
        )

        wallet_response_data_type_0.additional_properties = d
        return wallet_response_data_type_0

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
