from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

T = TypeVar("T", bound="SamsungPayMerchantPaymentInformation")


@_attrs_define
class SamsungPayMerchantPaymentInformation:
    """
    Attributes:
        name (str): Merchant name, this will be displayed on the Samsung Pay screen
        country_code (CountryAlpha2):
        url (Union[None, Unset, str]): Merchant domain that process payments, required for web payments
    """

    name: str
    country_code: CountryAlpha2
    url: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        country_code = self.country_code.value

        url: Union[None, Unset, str]
        if isinstance(self.url, Unset):
            url = UNSET
        else:
            url = self.url

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "country_code": country_code,
            }
        )
        if url is not UNSET:
            field_dict["url"] = url

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        name = d.pop("name")

        country_code = CountryAlpha2(d.pop("country_code"))

        def _parse_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        url = _parse_url(d.pop("url", UNSET))

        samsung_pay_merchant_payment_information = cls(
            name=name,
            country_code=country_code,
            url=url,
        )

        samsung_pay_merchant_payment_information.additional_properties = d
        return samsung_pay_merchant_payment_information

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
