from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.currency import Currency
from ..models.samsung_pay_amount_format import SamsungPayAmountFormat

T = TypeVar("T", bound="SamsungPayAmountDetails")


@_attrs_define
class SamsungPayAmountDetails:
    """
    Attributes:
        option (SamsungPayAmountFormat):
        currency_code (Currency): The three letter ISO currency code in uppercase. Eg: 'USD' for the United States
            Dollar.
        total (str): The total amount of the transaction Example: 38.02.
    """

    option: SamsungPayAmountFormat
    currency_code: Currency
    total: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        option = self.option.value

        currency_code = self.currency_code.value

        total = self.total

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "option": option,
                "currency_code": currency_code,
                "total": total,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        option = SamsungPayAmountFormat(d.pop("option"))

        currency_code = Currency(d.pop("currency_code"))

        total = d.pop("total")

        samsung_pay_amount_details = cls(
            option=option,
            currency_code=currency_code,
            total=total,
        )

        samsung_pay_amount_details.additional_properties = d
        return samsung_pay_amount_details

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
