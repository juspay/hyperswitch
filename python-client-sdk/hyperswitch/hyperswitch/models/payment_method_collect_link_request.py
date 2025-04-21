from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.enabled_payment_method import EnabledPaymentMethod


T = TypeVar("T", bound="PaymentMethodCollectLinkRequest")


@_attrs_define
class PaymentMethodCollectLinkRequest:
    """
    Attributes:
        customer_id (str): The unique identifier of the customer. Example: cus_92dnwed8s32bV9D8Snbiasd8v.
        pm_collect_link_id (Union[None, Unset, str]): The unique identifier for the collect link. Example:
            pm_collect_link_2bdacf398vwzq5n422S1.
        session_expiry (Union[None, Unset, int]): Will be used to expire client secret after certain amount of time to
            be supplied in seconds
            (900) for 15 mins Example: 900.
        return_url (Union[None, Unset, str]): Redirect to this URL post completion Example:
            https://sandbox.hyperswitch.io/payment_method/collect/pm_collect_link_2bdacf398vwzq5n422S1/status.
        enabled_payment_methods (Union[None, Unset, list['EnabledPaymentMethod']]): List of payment methods shown on
            collect UI Example: [{"payment_method": "bank_transfer", "payment_method_types": ["ach", "bacs"]}].
    """

    customer_id: str
    pm_collect_link_id: Union[None, Unset, str] = UNSET
    session_expiry: Union[None, Unset, int] = UNSET
    return_url: Union[None, Unset, str] = UNSET
    enabled_payment_methods: Union[None, Unset, list["EnabledPaymentMethod"]] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        customer_id = self.customer_id

        pm_collect_link_id: Union[None, Unset, str]
        if isinstance(self.pm_collect_link_id, Unset):
            pm_collect_link_id = UNSET
        else:
            pm_collect_link_id = self.pm_collect_link_id

        session_expiry: Union[None, Unset, int]
        if isinstance(self.session_expiry, Unset):
            session_expiry = UNSET
        else:
            session_expiry = self.session_expiry

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
                "customer_id": customer_id,
            }
        )
        if pm_collect_link_id is not UNSET:
            field_dict["pm_collect_link_id"] = pm_collect_link_id
        if session_expiry is not UNSET:
            field_dict["session_expiry"] = session_expiry
        if return_url is not UNSET:
            field_dict["return_url"] = return_url
        if enabled_payment_methods is not UNSET:
            field_dict["enabled_payment_methods"] = enabled_payment_methods

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.enabled_payment_method import EnabledPaymentMethod

        d = dict(src_dict)
        customer_id = d.pop("customer_id")

        def _parse_pm_collect_link_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        pm_collect_link_id = _parse_pm_collect_link_id(d.pop("pm_collect_link_id", UNSET))

        def _parse_session_expiry(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        session_expiry = _parse_session_expiry(d.pop("session_expiry", UNSET))

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

        payment_method_collect_link_request = cls(
            customer_id=customer_id,
            pm_collect_link_id=pm_collect_link_id,
            session_expiry=session_expiry,
            return_url=return_url,
            enabled_payment_methods=enabled_payment_methods,
        )

        payment_method_collect_link_request.additional_properties = d
        return payment_method_collect_link_request

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
