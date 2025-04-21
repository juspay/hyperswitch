from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.event_class import EventClass
from ..models.payment_connector_category import PaymentConnectorCategory
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.supported_payment_method import SupportedPaymentMethod


T = TypeVar("T", bound="ConnectorFeatureMatrixResponse")


@_attrs_define
class ConnectorFeatureMatrixResponse:
    """
    Attributes:
        name (str): The name of the connector
        supported_payment_methods (list['SupportedPaymentMethod']): The list of payment methods supported by the
            connector
        display_name (Union[None, Unset, str]): The display name of the connector
        description (Union[None, Unset, str]): The description of the connector
        category (Union[None, PaymentConnectorCategory, Unset]):
        supported_webhook_flows (Union[None, Unset, list[EventClass]]): The list of webhook flows supported by the
            connector
    """

    name: str
    supported_payment_methods: list["SupportedPaymentMethod"]
    display_name: Union[None, Unset, str] = UNSET
    description: Union[None, Unset, str] = UNSET
    category: Union[None, PaymentConnectorCategory, Unset] = UNSET
    supported_webhook_flows: Union[None, Unset, list[EventClass]] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        supported_payment_methods = []
        for supported_payment_methods_item_data in self.supported_payment_methods:
            supported_payment_methods_item = supported_payment_methods_item_data.to_dict()
            supported_payment_methods.append(supported_payment_methods_item)

        display_name: Union[None, Unset, str]
        if isinstance(self.display_name, Unset):
            display_name = UNSET
        else:
            display_name = self.display_name

        description: Union[None, Unset, str]
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        category: Union[None, Unset, str]
        if isinstance(self.category, Unset):
            category = UNSET
        elif isinstance(self.category, PaymentConnectorCategory):
            category = self.category.value
        else:
            category = self.category

        supported_webhook_flows: Union[None, Unset, list[str]]
        if isinstance(self.supported_webhook_flows, Unset):
            supported_webhook_flows = UNSET
        elif isinstance(self.supported_webhook_flows, list):
            supported_webhook_flows = []
            for supported_webhook_flows_type_0_item_data in self.supported_webhook_flows:
                supported_webhook_flows_type_0_item = supported_webhook_flows_type_0_item_data.value
                supported_webhook_flows.append(supported_webhook_flows_type_0_item)

        else:
            supported_webhook_flows = self.supported_webhook_flows

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "supported_payment_methods": supported_payment_methods,
            }
        )
        if display_name is not UNSET:
            field_dict["display_name"] = display_name
        if description is not UNSET:
            field_dict["description"] = description
        if category is not UNSET:
            field_dict["category"] = category
        if supported_webhook_flows is not UNSET:
            field_dict["supported_webhook_flows"] = supported_webhook_flows

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.supported_payment_method import SupportedPaymentMethod

        d = dict(src_dict)
        name = d.pop("name")

        supported_payment_methods = []
        _supported_payment_methods = d.pop("supported_payment_methods")
        for supported_payment_methods_item_data in _supported_payment_methods:
            supported_payment_methods_item = SupportedPaymentMethod.from_dict(supported_payment_methods_item_data)

            supported_payment_methods.append(supported_payment_methods_item)

        def _parse_display_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        display_name = _parse_display_name(d.pop("display_name", UNSET))

        def _parse_description(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        description = _parse_description(d.pop("description", UNSET))

        def _parse_category(data: object) -> Union[None, PaymentConnectorCategory, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                category_type_1 = PaymentConnectorCategory(data)

                return category_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentConnectorCategory, Unset], data)

        category = _parse_category(d.pop("category", UNSET))

        def _parse_supported_webhook_flows(data: object) -> Union[None, Unset, list[EventClass]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                supported_webhook_flows_type_0 = []
                _supported_webhook_flows_type_0 = data
                for supported_webhook_flows_type_0_item_data in _supported_webhook_flows_type_0:
                    supported_webhook_flows_type_0_item = EventClass(supported_webhook_flows_type_0_item_data)

                    supported_webhook_flows_type_0.append(supported_webhook_flows_type_0_item)

                return supported_webhook_flows_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[EventClass]], data)

        supported_webhook_flows = _parse_supported_webhook_flows(d.pop("supported_webhook_flows", UNSET))

        connector_feature_matrix_response = cls(
            name=name,
            supported_payment_methods=supported_payment_methods,
            display_name=display_name,
            description=description,
            category=category,
            supported_webhook_flows=supported_webhook_flows,
        )

        connector_feature_matrix_response.additional_properties = d
        return connector_feature_matrix_response

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
