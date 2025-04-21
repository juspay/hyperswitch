from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="GooglePayAssuranceDetails")


@_attrs_define
class GooglePayAssuranceDetails:
    """
    Attributes:
        card_holder_authenticated (bool): indicates that Cardholder possession validation has been performed
        account_verified (bool): indicates that identification and verifications (ID&V) was performed
    """

    card_holder_authenticated: bool
    account_verified: bool
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        card_holder_authenticated = self.card_holder_authenticated

        account_verified = self.account_verified

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "card_holder_authenticated": card_holder_authenticated,
                "account_verified": account_verified,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        card_holder_authenticated = d.pop("card_holder_authenticated")

        account_verified = d.pop("account_verified")

        google_pay_assurance_details = cls(
            card_holder_authenticated=card_holder_authenticated,
            account_verified=account_verified,
        )

        google_pay_assurance_details.additional_properties = d
        return google_pay_assurance_details

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
