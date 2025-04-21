from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

T = TypeVar("T", bound="PayLaterDataType0KlarnaRedirect")


@_attrs_define
class PayLaterDataType0KlarnaRedirect:
    """For KlarnaRedirect as PayLater Option

    Attributes:
        billing_email (Union[None, Unset, str]): The billing email
        billing_country (Union[CountryAlpha2, None, Unset]):
    """

    billing_email: Union[None, Unset, str] = UNSET
    billing_country: Union[CountryAlpha2, None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        billing_email: Union[None, Unset, str]
        if isinstance(self.billing_email, Unset):
            billing_email = UNSET
        else:
            billing_email = self.billing_email

        billing_country: Union[None, Unset, str]
        if isinstance(self.billing_country, Unset):
            billing_country = UNSET
        elif isinstance(self.billing_country, CountryAlpha2):
            billing_country = self.billing_country.value
        else:
            billing_country = self.billing_country

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if billing_email is not UNSET:
            field_dict["billing_email"] = billing_email
        if billing_country is not UNSET:
            field_dict["billing_country"] = billing_country

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

        def _parse_billing_country(data: object) -> Union[CountryAlpha2, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                billing_country_type_1 = CountryAlpha2(data)

                return billing_country_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CountryAlpha2, None, Unset], data)

        billing_country = _parse_billing_country(d.pop("billing_country", UNSET))

        pay_later_data_type_0_klarna_redirect = cls(
            billing_email=billing_email,
            billing_country=billing_country,
        )

        pay_later_data_type_0_klarna_redirect.additional_properties = d
        return pay_later_data_type_0_klarna_redirect

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
