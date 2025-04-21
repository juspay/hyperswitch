from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="RequestSurchargeDetails")


@_attrs_define
class RequestSurchargeDetails:
    """Details of surcharge applied on this payment, if applicable

    Attributes:
        surcharge_amount (int):  Example: 6540.
        tax_amount (Union[None, Unset, int]):
    """

    surcharge_amount: int
    tax_amount: Union[None, Unset, int] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        surcharge_amount = self.surcharge_amount

        tax_amount: Union[None, Unset, int]
        if isinstance(self.tax_amount, Unset):
            tax_amount = UNSET
        else:
            tax_amount = self.tax_amount

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "surcharge_amount": surcharge_amount,
            }
        )
        if tax_amount is not UNSET:
            field_dict["tax_amount"] = tax_amount

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        surcharge_amount = d.pop("surcharge_amount")

        def _parse_tax_amount(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        tax_amount = _parse_tax_amount(d.pop("tax_amount", UNSET))

        request_surcharge_details = cls(
            surcharge_amount=surcharge_amount,
            tax_amount=tax_amount,
        )

        request_surcharge_details.additional_properties = d
        return request_surcharge_details

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
