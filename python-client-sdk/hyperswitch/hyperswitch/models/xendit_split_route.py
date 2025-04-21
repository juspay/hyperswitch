from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.currency import Currency
from ..types import UNSET, Unset

T = TypeVar("T", bound="XenditSplitRoute")


@_attrs_define
class XenditSplitRoute:
    """Fee information to be charged on the payment being collected via xendit

    Attributes:
        currency (Currency): The three letter ISO currency code in uppercase. Eg: 'USD' for the United States Dollar.
        destination_account_id (str): ID of the destination account where the amount will be routed to
        reference_id (str): Reference ID which acts as an identifier of the route itself
        flat_amount (Union[None, Unset, int]):
        percent_amount (Union[None, Unset, int]): Amount of payments to be split, using a percent rate as unit
    """

    currency: Currency
    destination_account_id: str
    reference_id: str
    flat_amount: Union[None, Unset, int] = UNSET
    percent_amount: Union[None, Unset, int] = UNSET

    def to_dict(self) -> dict[str, Any]:
        currency = self.currency.value

        destination_account_id = self.destination_account_id

        reference_id = self.reference_id

        flat_amount: Union[None, Unset, int]
        if isinstance(self.flat_amount, Unset):
            flat_amount = UNSET
        else:
            flat_amount = self.flat_amount

        percent_amount: Union[None, Unset, int]
        if isinstance(self.percent_amount, Unset):
            percent_amount = UNSET
        else:
            percent_amount = self.percent_amount

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "currency": currency,
                "destination_account_id": destination_account_id,
                "reference_id": reference_id,
            }
        )
        if flat_amount is not UNSET:
            field_dict["flat_amount"] = flat_amount
        if percent_amount is not UNSET:
            field_dict["percent_amount"] = percent_amount

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        currency = Currency(d.pop("currency"))

        destination_account_id = d.pop("destination_account_id")

        reference_id = d.pop("reference_id")

        def _parse_flat_amount(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        flat_amount = _parse_flat_amount(d.pop("flat_amount", UNSET))

        def _parse_percent_amount(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        percent_amount = _parse_percent_amount(d.pop("percent_amount", UNSET))

        xendit_split_route = cls(
            currency=currency,
            destination_account_id=destination_account_id,
            reference_id=reference_id,
            flat_amount=flat_amount,
            percent_amount=percent_amount,
        )

        return xendit_split_route
