from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="BankRedirectBilling")


@_attrs_define
class BankRedirectBilling:
    """
    Attributes:
        billing_name (str): The name for which billing is issued Example: John Doe.
        email (str): The billing email for bank redirect Example: example@example.com.
    """

    billing_name: str
    email: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        billing_name = self.billing_name

        email = self.email

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "billing_name": billing_name,
                "email": email,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        billing_name = d.pop("billing_name")

        email = d.pop("email")

        bank_redirect_billing = cls(
            billing_name=billing_name,
            email=email,
        )

        bank_redirect_billing.additional_properties = d
        return bank_redirect_billing

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
