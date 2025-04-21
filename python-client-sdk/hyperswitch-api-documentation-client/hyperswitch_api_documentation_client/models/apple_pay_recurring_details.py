from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.apple_pay_regular_billing_details import ApplePayRegularBillingDetails


T = TypeVar("T", bound="ApplePayRecurringDetails")


@_attrs_define
class ApplePayRecurringDetails:
    """
    Attributes:
        payment_description (str): A description of the recurring payment that Apple Pay displays to the user in the
            payment sheet
        regular_billing (ApplePayRegularBillingDetails):
        management_url (str): A URL to a web page where the user can update or delete the payment method for the
            recurring payment Example: https://hyperswitch.io.
        billing_agreement (Union[None, Unset, str]): A localized billing agreement that the payment sheet displays to
            the user before the user authorizes the payment
    """

    payment_description: str
    regular_billing: "ApplePayRegularBillingDetails"
    management_url: str
    billing_agreement: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payment_description = self.payment_description

        regular_billing = self.regular_billing.to_dict()

        management_url = self.management_url

        billing_agreement: Union[None, Unset, str]
        if isinstance(self.billing_agreement, Unset):
            billing_agreement = UNSET
        else:
            billing_agreement = self.billing_agreement

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_description": payment_description,
                "regular_billing": regular_billing,
                "management_url": management_url,
            }
        )
        if billing_agreement is not UNSET:
            field_dict["billing_agreement"] = billing_agreement

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.apple_pay_regular_billing_details import ApplePayRegularBillingDetails

        d = dict(src_dict)
        payment_description = d.pop("payment_description")

        regular_billing = ApplePayRegularBillingDetails.from_dict(d.pop("regular_billing"))

        management_url = d.pop("management_url")

        def _parse_billing_agreement(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        billing_agreement = _parse_billing_agreement(d.pop("billing_agreement", UNSET))

        apple_pay_recurring_details = cls(
            payment_description=payment_description,
            regular_billing=regular_billing,
            management_url=management_url,
            billing_agreement=billing_agreement,
        )

        apple_pay_recurring_details.additional_properties = d
        return apple_pay_recurring_details

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
