from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.bancontact_bank_redirect_additional_data import BancontactBankRedirectAdditionalData


T = TypeVar("T", bound="BankRedirectDetailsType0")


@_attrs_define
class BankRedirectDetailsType0:
    """
    Attributes:
        bancontact_card (BancontactBankRedirectAdditionalData):
    """

    bancontact_card: "BancontactBankRedirectAdditionalData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        bancontact_card = self.bancontact_card.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "BancontactCard": bancontact_card,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.bancontact_bank_redirect_additional_data import BancontactBankRedirectAdditionalData

        d = dict(src_dict)
        bancontact_card = BancontactBankRedirectAdditionalData.from_dict(d.pop("BancontactCard"))

        bank_redirect_details_type_0 = cls(
            bancontact_card=bancontact_card,
        )

        bank_redirect_details_type_0.additional_properties = d
        return bank_redirect_details_type_0

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
