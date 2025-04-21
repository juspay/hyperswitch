from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="PayLaterDataType3AfterpayClearpayRedirect")


@_attrs_define
class PayLaterDataType3AfterpayClearpayRedirect:
    """For AfterpayClearpay redirect as PayLater Option

    Attributes:
        billing_email (Union[None, Unset, str]): The billing email
        billing_name (Union[None, Unset, str]): The billing name
    """

    billing_email: Union[None, Unset, str] = UNSET
    billing_name: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        billing_email: Union[None, Unset, str]
        if isinstance(self.billing_email, Unset):
            billing_email = UNSET
        else:
            billing_email = self.billing_email

        billing_name: Union[None, Unset, str]
        if isinstance(self.billing_name, Unset):
            billing_name = UNSET
        else:
            billing_name = self.billing_name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if billing_email is not UNSET:
            field_dict["billing_email"] = billing_email
        if billing_name is not UNSET:
            field_dict["billing_name"] = billing_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_billing_email(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        billing_email = _parse_billing_email(d.pop("billing_email", UNSET))

        def _parse_billing_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        billing_name = _parse_billing_name(d.pop("billing_name", UNSET))

        pay_later_data_type_3_afterpay_clearpay_redirect = cls(
            billing_email=billing_email,
            billing_name=billing_name,
        )

        pay_later_data_type_3_afterpay_clearpay_redirect.additional_properties = d
        return pay_later_data_type_3_afterpay_clearpay_redirect

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
