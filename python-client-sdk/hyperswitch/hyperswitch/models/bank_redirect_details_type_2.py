from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.giropay_bank_redirect_additional_data import GiropayBankRedirectAdditionalData


T = TypeVar("T", bound="BankRedirectDetailsType2")


@_attrs_define
class BankRedirectDetailsType2:
    """
    Attributes:
        giropay (GiropayBankRedirectAdditionalData):
    """

    giropay: "GiropayBankRedirectAdditionalData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        giropay = self.giropay.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "Giropay": giropay,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.giropay_bank_redirect_additional_data import GiropayBankRedirectAdditionalData

        d = dict(src_dict)
        giropay = GiropayBankRedirectAdditionalData.from_dict(d.pop("Giropay"))

        bank_redirect_details_type_2 = cls(
            giropay=giropay,
        )

        bank_redirect_details_type_2.additional_properties = d
        return bank_redirect_details_type_2

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
