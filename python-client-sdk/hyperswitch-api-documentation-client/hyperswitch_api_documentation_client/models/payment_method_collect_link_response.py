import datetime
from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.enabled_payment_method import EnabledPaymentMethod


T = TypeVar("T", bound="PaymentMethodCollectLinkResponse")


@_attrs_define
class PaymentMethodCollectLinkResponse:
    """
    Attributes:
        pm_collect_link_id (str): The unique identifier for the collect link. Example:
            pm_collect_link_2bdacf398vwzq5n422S1.
        customer_id (str): The unique identifier of the customer. Example: cus_92dnwed8s32bV9D8Snbiasd8v.
        expiry (datetime.datetime): Time when this link will be expired in ISO8601 format Example:
            2025-01-18T11:04:09.922Z.
        link (str): URL to the form's link generated for collecting payment method details. Example:
            https://sandbox.hyperswitch.io/payment_method/collect/pm_collect_link_2bdacf398vwzq5n422S1.
        logo (Union[None, Unset, str]): Merchant's display logo Example: https://hyperswitch.io/favicon.ico.
        merchant_name (Union[None, Unset, str]): Custom merchant name for the link Example: Hyperswitch.
        theme (Union[None, Unset, str]): Primary color to be used in the form represented in hex format Example:
            #4285F4.
        return_url (Union[None, Unset, str]): Redirect to this URL post completion Example:
            https://sandbox.hyperswitch.io/payment_method/collect/pm_collect_link_2bdacf398vwzq5n422S1/status.
        enabled_payment_methods (Union[None, Unset, list['EnabledPaymentMethod']]): List of payment methods shown on
            collect UI Example: [{"payment_method": "bank_transfer", "payment_method_types": ["ach", "bacs"]}].
    """

    pm_collect_link_id: str
    customer_id: str
    expiry: datetime.datetime
    link: str
    logo: Union[None, Unset, str] = UNSET
    merchant_name: Union[None, Unset, str] = UNSET
    theme: Union[None, Unset, str] = UNSET
    return_url: Union[None, Unset, str] = UNSET
    enabled_payment_methods: Union[None, Unset, list["EnabledPaymentMethod"]] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        pm_collect_link_id = self.pm_collect_link_id

        customer_id = self.customer_id

        expiry = self.expiry.isoformat()

        link = self.link

        logo: Union[None, Unset, str]
        if isinstance(self.logo, Unset):
            logo = UNSET
        else:
            logo = self.logo

        merchant_name: Union[None, Unset, str]
        if isinstance(self.merchant_name, Unset):
            merchant_name = UNSET
        else:
            merchant_name = self.merchant_name

        theme: Union[None, Unset, str]
        if isinstance(self.theme, Unset):
            theme = UNSET
        else:
            theme = self.theme

        return_url: Union[None, Unset, str]
        if isinstance(self.return_url, Unset):
            return_url = UNSET
        else:
            return_url = self.return_url

        enabled_payment_methods: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.enabled_payment_methods, Unset):
            enabled_payment_methods = UNSET
        elif isinstance(self.enabled_payment_methods, list):
            enabled_payment_methods = []
            for enabled_payment_methods_type_0_item_data in self.enabled_payment_methods:
                enabled_payment_methods_type_0_item = enabled_payment_methods_type_0_item_data.to_dict()
                enabled_payment_methods.append(enabled_payment_methods_type_0_item)

        else:
            enabled_payment_methods = self.enabled_payment_methods

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "pm_collect_link_id": pm_collect_link_id,
                "customer_id": customer_id,
                "expiry": expiry,
                "link": link,
            }
        )
        if logo is not UNSET:
            field_dict["logo"] = logo
        if merchant_name is not UNSET:
            field_dict["merchant_name"] = merchant_name
        if theme is not UNSET:
            field_dict["theme"] = theme
        if return_url is not UNSET:
            field_dict["return_url"] = return_url
        if enabled_payment_methods is not UNSET:
            field_dict["enabled_payment_methods"] = enabled_payment_methods

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.enabled_payment_method import EnabledPaymentMethod

        d = dict(src_dict)
        pm_collect_link_id = d.pop("pm_collect_link_id")

        customer_id = d.pop("customer_id")

        expiry = isoparse(d.pop("expiry"))

        link = d.pop("link")

        def _parse_logo(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        logo = _parse_logo(d.pop("logo", UNSET))

        def _parse_merchant_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_name = _parse_merchant_name(d.pop("merchant_name", UNSET))

        def _parse_theme(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        theme = _parse_theme(d.pop("theme", UNSET))

        def _parse_return_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        return_url = _parse_return_url(d.pop("return_url", UNSET))

        def _parse_enabled_payment_methods(data: object) -> Union[None, Unset, list["EnabledPaymentMethod"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                enabled_payment_methods_type_0 = []
                _enabled_payment_methods_type_0 = data
                for enabled_payment_methods_type_0_item_data in _enabled_payment_methods_type_0:
                    enabled_payment_methods_type_0_item = EnabledPaymentMethod.from_dict(
                        enabled_payment_methods_type_0_item_data
                    )

                    enabled_payment_methods_type_0.append(enabled_payment_methods_type_0_item)

                return enabled_payment_methods_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["EnabledPaymentMethod"]], data)

        enabled_payment_methods = _parse_enabled_payment_methods(d.pop("enabled_payment_methods", UNSET))

        payment_method_collect_link_response = cls(
            pm_collect_link_id=pm_collect_link_id,
            customer_id=customer_id,
            expiry=expiry,
            link=link,
            logo=logo,
            merchant_name=merchant_name,
            theme=theme,
            return_url=return_url,
            enabled_payment_methods=enabled_payment_methods,
        )

        payment_method_collect_link_response.additional_properties = d
        return payment_method_collect_link_response

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
