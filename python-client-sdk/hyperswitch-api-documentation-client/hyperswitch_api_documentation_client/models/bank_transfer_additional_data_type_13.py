from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.local_bank_transfer_additional_data import LocalBankTransferAdditionalData


T = TypeVar("T", bound="BankTransferAdditionalDataType13")


@_attrs_define
class BankTransferAdditionalDataType13:
    """
    Attributes:
        local_bank_transfer (LocalBankTransferAdditionalData):
    """

    local_bank_transfer: "LocalBankTransferAdditionalData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        local_bank_transfer = self.local_bank_transfer.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "local_bank_transfer": local_bank_transfer,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.local_bank_transfer_additional_data import LocalBankTransferAdditionalData

        d = dict(src_dict)
        local_bank_transfer = LocalBankTransferAdditionalData.from_dict(d.pop("local_bank_transfer"))

        bank_transfer_additional_data_type_13 = cls(
            local_bank_transfer=local_bank_transfer,
        )

        bank_transfer_additional_data_type_13.additional_properties = d
        return bank_transfer_additional_data_type_13

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
