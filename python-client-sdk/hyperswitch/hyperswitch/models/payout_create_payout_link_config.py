from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.ui_widget_form_layout import UIWidgetFormLayout
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.enabled_payment_method import EnabledPaymentMethod


T = TypeVar("T", bound="PayoutCreatePayoutLinkConfig")


@_attrs_define
class PayoutCreatePayoutLinkConfig:
    """Custom payout link config for the particular payout, if payout link is to be generated.

    Attributes:
        payout_link_id (Union[None, Unset, str]): The unique identifier for the collect link. Example:
            pm_collect_link_2bdacf398vwzq5n422S1.
        enabled_payment_methods (Union[None, Unset, list['EnabledPaymentMethod']]): List of payout methods shown on
            collect UI Example: [{"payment_method": "bank_transfer", "payment_method_types": ["ach", "bacs"]}].
        form_layout (Union[None, UIWidgetFormLayout, Unset]):
        test_mode (Union[None, Unset, bool]): `test_mode` allows for opening payout links without any restrictions. This
            removes
            - domain name validations
            - check for making sure link is accessed within an iframe
    """

    payout_link_id: Union[None, Unset, str] = UNSET
    enabled_payment_methods: Union[None, Unset, list["EnabledPaymentMethod"]] = UNSET
    form_layout: Union[None, UIWidgetFormLayout, Unset] = UNSET
    test_mode: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payout_link_id: Union[None, Unset, str]
        if isinstance(self.payout_link_id, Unset):
            payout_link_id = UNSET
        else:
            payout_link_id = self.payout_link_id

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

        form_layout: Union[None, Unset, str]
        if isinstance(self.form_layout, Unset):
            form_layout = UNSET
        elif isinstance(self.form_layout, UIWidgetFormLayout):
            form_layout = self.form_layout.value
        else:
            form_layout = self.form_layout

        test_mode: Union[None, Unset, bool]
        if isinstance(self.test_mode, Unset):
            test_mode = UNSET
        else:
            test_mode = self.test_mode

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if payout_link_id is not UNSET:
            field_dict["payout_link_id"] = payout_link_id
        if enabled_payment_methods is not UNSET:
            field_dict["enabled_payment_methods"] = enabled_payment_methods
        if form_layout is not UNSET:
            field_dict["form_layout"] = form_layout
        if test_mode is not UNSET:
            field_dict["test_mode"] = test_mode

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.enabled_payment_method import EnabledPaymentMethod

        d = dict(src_dict)

        def _parse_payout_link_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payout_link_id = _parse_payout_link_id(d.pop("payout_link_id", UNSET))

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

        def _parse_form_layout(data: object) -> Union[None, UIWidgetFormLayout, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                form_layout_type_1 = UIWidgetFormLayout(data)

                return form_layout_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, UIWidgetFormLayout, Unset], data)

        form_layout = _parse_form_layout(d.pop("form_layout", UNSET))

        def _parse_test_mode(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        test_mode = _parse_test_mode(d.pop("test_mode", UNSET))

        payout_create_payout_link_config = cls(
            payout_link_id=payout_link_id,
            enabled_payment_methods=enabled_payment_methods,
            form_layout=form_layout,
            test_mode=test_mode,
        )

        payout_create_payout_link_config.additional_properties = d
        return payout_create_payout_link_config

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
