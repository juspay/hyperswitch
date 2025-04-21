from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.wallet_additional_data_for_card import WalletAdditionalDataForCard


T = TypeVar("T", bound="WalletResponseDataType2")


@_attrs_define
class WalletResponseDataType2:
    """
    Attributes:
        samsung_pay (WalletAdditionalDataForCard):
    """

    samsung_pay: "WalletAdditionalDataForCard"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        samsung_pay = self.samsung_pay.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "samsung_pay": samsung_pay,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.wallet_additional_data_for_card import WalletAdditionalDataForCard

        d = dict(src_dict)
        samsung_pay = WalletAdditionalDataForCard.from_dict(d.pop("samsung_pay"))

        wallet_response_data_type_2 = cls(
            samsung_pay=samsung_pay,
        )

        wallet_response_data_type_2.additional_properties = d
        return wallet_response_data_type_2

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
