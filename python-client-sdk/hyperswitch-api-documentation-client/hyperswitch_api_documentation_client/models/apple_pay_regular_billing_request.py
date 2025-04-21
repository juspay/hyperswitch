import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..models.apple_pay_payment_timing import ApplePayPaymentTiming
from ..models.recurring_payment_interval_unit import RecurringPaymentIntervalUnit
from ..types import UNSET, Unset

T = TypeVar("T", bound="ApplePayRegularBillingRequest")


@_attrs_define
class ApplePayRegularBillingRequest:
    """
    Attributes:
        amount (str): The amount of the recurring payment Example: 38.02.
        label (str): The label that Apple Pay displays to the user in the payment sheet with the recurring details
        payment_timing (ApplePayPaymentTiming):
        recurring_payment_start_date (Union[None, Unset, datetime.datetime]): The date of the first payment
        recurring_payment_end_date (Union[None, Unset, datetime.datetime]): The date of the final payment
        recurring_payment_interval_unit (Union[None, RecurringPaymentIntervalUnit, Unset]):
        recurring_payment_interval_count (Union[None, Unset, int]): The number of interval units that make up the total
            payment interval
    """

    amount: str
    label: str
    payment_timing: ApplePayPaymentTiming
    recurring_payment_start_date: Union[None, Unset, datetime.datetime] = UNSET
    recurring_payment_end_date: Union[None, Unset, datetime.datetime] = UNSET
    recurring_payment_interval_unit: Union[None, RecurringPaymentIntervalUnit, Unset] = UNSET
    recurring_payment_interval_count: Union[None, Unset, int] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        amount = self.amount

        label = self.label

        payment_timing = self.payment_timing.value

        recurring_payment_start_date: Union[None, Unset, str]
        if isinstance(self.recurring_payment_start_date, Unset):
            recurring_payment_start_date = UNSET
        elif isinstance(self.recurring_payment_start_date, datetime.datetime):
            recurring_payment_start_date = self.recurring_payment_start_date.isoformat()
        else:
            recurring_payment_start_date = self.recurring_payment_start_date

        recurring_payment_end_date: Union[None, Unset, str]
        if isinstance(self.recurring_payment_end_date, Unset):
            recurring_payment_end_date = UNSET
        elif isinstance(self.recurring_payment_end_date, datetime.datetime):
            recurring_payment_end_date = self.recurring_payment_end_date.isoformat()
        else:
            recurring_payment_end_date = self.recurring_payment_end_date

        recurring_payment_interval_unit: Union[None, Unset, str]
        if isinstance(self.recurring_payment_interval_unit, Unset):
            recurring_payment_interval_unit = UNSET
        elif isinstance(self.recurring_payment_interval_unit, RecurringPaymentIntervalUnit):
            recurring_payment_interval_unit = self.recurring_payment_interval_unit.value
        else:
            recurring_payment_interval_unit = self.recurring_payment_interval_unit

        recurring_payment_interval_count: Union[None, Unset, int]
        if isinstance(self.recurring_payment_interval_count, Unset):
            recurring_payment_interval_count = UNSET
        else:
            recurring_payment_interval_count = self.recurring_payment_interval_count

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "amount": amount,
                "label": label,
                "payment_timing": payment_timing,
            }
        )
        if recurring_payment_start_date is not UNSET:
            field_dict["recurring_payment_start_date"] = recurring_payment_start_date
        if recurring_payment_end_date is not UNSET:
            field_dict["recurring_payment_end_date"] = recurring_payment_end_date
        if recurring_payment_interval_unit is not UNSET:
            field_dict["recurring_payment_interval_unit"] = recurring_payment_interval_unit
        if recurring_payment_interval_count is not UNSET:
            field_dict["recurring_payment_interval_count"] = recurring_payment_interval_count

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        amount = d.pop("amount")

        label = d.pop("label")

        payment_timing = ApplePayPaymentTiming(d.pop("payment_timing"))

        def _parse_recurring_payment_start_date(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                recurring_payment_start_date_type_0 = isoparse(data)

                return recurring_payment_start_date_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        recurring_payment_start_date = _parse_recurring_payment_start_date(d.pop("recurring_payment_start_date", UNSET))

        def _parse_recurring_payment_end_date(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                recurring_payment_end_date_type_0 = isoparse(data)

                return recurring_payment_end_date_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        recurring_payment_end_date = _parse_recurring_payment_end_date(d.pop("recurring_payment_end_date", UNSET))

        def _parse_recurring_payment_interval_unit(data: object) -> Union[None, RecurringPaymentIntervalUnit, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                recurring_payment_interval_unit_type_1 = RecurringPaymentIntervalUnit(data)

                return recurring_payment_interval_unit_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, RecurringPaymentIntervalUnit, Unset], data)

        recurring_payment_interval_unit = _parse_recurring_payment_interval_unit(
            d.pop("recurring_payment_interval_unit", UNSET)
        )

        def _parse_recurring_payment_interval_count(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        recurring_payment_interval_count = _parse_recurring_payment_interval_count(
            d.pop("recurring_payment_interval_count", UNSET)
        )

        apple_pay_regular_billing_request = cls(
            amount=amount,
            label=label,
            payment_timing=payment_timing,
            recurring_payment_start_date=recurring_payment_start_date,
            recurring_payment_end_date=recurring_payment_end_date,
            recurring_payment_interval_unit=recurring_payment_interval_unit,
            recurring_payment_interval_count=recurring_payment_interval_count,
        )

        apple_pay_regular_billing_request.additional_properties = d
        return apple_pay_regular_billing_request

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
