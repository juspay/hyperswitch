from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.sepa_bank_debit_additional_data import SepaBankDebitAdditionalData


T = TypeVar("T", bound="BankDebitAdditionalDataType3")


@_attrs_define
class BankDebitAdditionalDataType3:
    """
    Attributes:
        sepa (SepaBankDebitAdditionalData):
    """

    sepa: "SepaBankDebitAdditionalData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        sepa = self.sepa.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "sepa": sepa,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.sepa_bank_debit_additional_data import SepaBankDebitAdditionalData

        d = dict(src_dict)
        sepa = SepaBankDebitAdditionalData.from_dict(d.pop("sepa"))

        bank_debit_additional_data_type_3 = cls(
            sepa=sepa,
        )

        bank_debit_additional_data_type_3.additional_properties = d
        return bank_debit_additional_data_type_3

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
