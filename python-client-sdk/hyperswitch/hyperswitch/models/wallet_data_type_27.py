from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.swish_qr_data import SwishQrData


T = TypeVar("T", bound="WalletDataType27")


@_attrs_define
class WalletDataType27:
    """
    Attributes:
        swish_qr (SwishQrData):
    """

    swish_qr: "SwishQrData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        swish_qr = self.swish_qr.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "swish_qr": swish_qr,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.swish_qr_data import SwishQrData

        d = dict(src_dict)
        swish_qr = SwishQrData.from_dict(d.pop("swish_qr"))

        wallet_data_type_27 = cls(
            swish_qr=swish_qr,
        )

        wallet_data_type_27.additional_properties = d
        return wallet_data_type_27

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
