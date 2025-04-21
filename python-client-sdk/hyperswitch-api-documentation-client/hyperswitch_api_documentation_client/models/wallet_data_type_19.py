from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.paze_wallet_data import PazeWalletData


T = TypeVar("T", bound="WalletDataType19")


@_attrs_define
class WalletDataType19:
    """
    Attributes:
        paze (PazeWalletData):
    """

    paze: "PazeWalletData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        paze = self.paze.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "paze": paze,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.paze_wallet_data import PazeWalletData

        d = dict(src_dict)
        paze = PazeWalletData.from_dict(d.pop("paze"))

        wallet_data_type_19 = cls(
            paze=paze,
        )

        wallet_data_type_19.additional_properties = d
        return wallet_data_type_19

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
