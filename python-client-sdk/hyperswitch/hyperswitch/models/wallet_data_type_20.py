from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.samsung_pay_wallet_data import SamsungPayWalletData


T = TypeVar("T", bound="WalletDataType20")


@_attrs_define
class WalletDataType20:
    """
    Attributes:
        samsung_pay (SamsungPayWalletData):
    """

    samsung_pay: "SamsungPayWalletData"
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
        from ..models.samsung_pay_wallet_data import SamsungPayWalletData

        d = dict(src_dict)
        samsung_pay = SamsungPayWalletData.from_dict(d.pop("samsung_pay"))

        wallet_data_type_20 = cls(
            samsung_pay=samsung_pay,
        )

        wallet_data_type_20.additional_properties = d
        return wallet_data_type_20

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
