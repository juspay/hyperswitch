from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.country_alpha_2 import CountryAlpha2
from ..models.currency import Currency

T = TypeVar("T", bound="GpayTransactionInfo")


@_attrs_define
class GpayTransactionInfo:
    """
    Attributes:
        country_code (CountryAlpha2):
        currency_code (Currency): The three letter ISO currency code in uppercase. Eg: 'USD' for the United States
            Dollar.
        total_price_status (str): The total price status (ex: 'FINAL')
        total_price (str): The total price Example: 38.02.
    """

    country_code: CountryAlpha2
    currency_code: Currency
    total_price_status: str
    total_price: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        country_code = self.country_code.value

        currency_code = self.currency_code.value

        total_price_status = self.total_price_status

        total_price = self.total_price

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "country_code": country_code,
                "currency_code": currency_code,
                "total_price_status": total_price_status,
                "total_price": total_price,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        country_code = CountryAlpha2(d.pop("country_code"))

        currency_code = Currency(d.pop("currency_code"))

        total_price_status = d.pop("total_price_status")

        total_price = d.pop("total_price")

        gpay_transaction_info = cls(
            country_code=country_code,
            currency_code=currency_code,
            total_price_status=total_price_status,
            total_price=total_price,
        )

        gpay_transaction_info.additional_properties = d
        return gpay_transaction_info

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
