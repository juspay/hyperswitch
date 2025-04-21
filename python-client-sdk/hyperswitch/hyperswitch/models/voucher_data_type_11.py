from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.jcs_voucher_data import JCSVoucherData


T = TypeVar("T", bound="VoucherDataType11")


@_attrs_define
class VoucherDataType11:
    """
    Attributes:
        family_mart (JCSVoucherData):
    """

    family_mart: "JCSVoucherData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        family_mart = self.family_mart.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "family_mart": family_mart,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.jcs_voucher_data import JCSVoucherData

        d = dict(src_dict)
        family_mart = JCSVoucherData.from_dict(d.pop("family_mart"))

        voucher_data_type_11 = cls(
            family_mart=family_mart,
        )

        voucher_data_type_11.additional_properties = d
        return voucher_data_type_11

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
