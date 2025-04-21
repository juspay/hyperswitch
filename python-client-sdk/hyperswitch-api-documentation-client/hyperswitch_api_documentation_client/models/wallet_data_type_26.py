from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.cashapp_qr import CashappQr


T = TypeVar("T", bound="WalletDataType26")


@_attrs_define
class WalletDataType26:
    """
    Attributes:
        cashapp_qr (CashappQr):
    """

    cashapp_qr: "CashappQr"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        cashapp_qr = self.cashapp_qr.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "cashapp_qr": cashapp_qr,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.cashapp_qr import CashappQr

        d = dict(src_dict)
        cashapp_qr = CashappQr.from_dict(d.pop("cashapp_qr"))

        wallet_data_type_26 = cls(
            cashapp_qr=cashapp_qr,
        )

        wallet_data_type_26.additional_properties = d
        return wallet_data_type_26

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
