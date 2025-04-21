from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.boleto_voucher_data import BoletoVoucherData


T = TypeVar("T", bound="VoucherDataType0")


@_attrs_define
class VoucherDataType0:
    """
    Attributes:
        boleto (BoletoVoucherData):
    """

    boleto: "BoletoVoucherData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        boleto = self.boleto.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "boleto": boleto,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.boleto_voucher_data import BoletoVoucherData

        d = dict(src_dict)
        boleto = BoletoVoucherData.from_dict(d.pop("boleto"))

        voucher_data_type_0 = cls(
            boleto=boleto,
        )

        voucher_data_type_0.additional_properties = d
        return voucher_data_type_0

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
