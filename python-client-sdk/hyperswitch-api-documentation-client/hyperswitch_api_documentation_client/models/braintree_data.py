from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="BraintreeData")


@_attrs_define
class BraintreeData:
    """
    Attributes:
        merchant_account_id (str): Information about the merchant_account_id that merchant wants to specify at connector
            level.
        merchant_config_currency (str): Information about the merchant_config_currency that merchant wants to specify at
            connector level.
    """

    merchant_account_id: str
    merchant_config_currency: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        merchant_account_id = self.merchant_account_id

        merchant_config_currency = self.merchant_config_currency

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "merchant_account_id": merchant_account_id,
                "merchant_config_currency": merchant_config_currency,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        merchant_account_id = d.pop("merchant_account_id")

        merchant_config_currency = d.pop("merchant_config_currency")

        braintree_data = cls(
            merchant_account_id=merchant_account_id,
            merchant_config_currency=merchant_config_currency,
        )

        braintree_data.additional_properties = d
        return braintree_data

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
