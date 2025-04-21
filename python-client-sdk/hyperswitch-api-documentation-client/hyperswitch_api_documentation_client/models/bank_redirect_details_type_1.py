from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.blik_bank_redirect_additional_data import BlikBankRedirectAdditionalData


T = TypeVar("T", bound="BankRedirectDetailsType1")


@_attrs_define
class BankRedirectDetailsType1:
    """
    Attributes:
        blik (BlikBankRedirectAdditionalData):
    """

    blik: "BlikBankRedirectAdditionalData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        blik = self.blik.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "Blik": blik,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.blik_bank_redirect_additional_data import BlikBankRedirectAdditionalData

        d = dict(src_dict)
        blik = BlikBankRedirectAdditionalData.from_dict(d.pop("Blik"))

        bank_redirect_details_type_1 = cls(
            blik=blik,
        )

        bank_redirect_details_type_1.additional_properties = d
        return bank_redirect_details_type_1

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
