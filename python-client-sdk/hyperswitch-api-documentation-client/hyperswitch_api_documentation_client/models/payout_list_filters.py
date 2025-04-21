from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.currency import Currency
from ..models.payout_connectors import PayoutConnectors
from ..models.payout_status import PayoutStatus
from ..models.payout_type import PayoutType

T = TypeVar("T", bound="PayoutListFilters")


@_attrs_define
class PayoutListFilters:
    """
    Attributes:
        connector (list[PayoutConnectors]): The list of available connector filters
        currency (list[Currency]): The list of available currency filters
        status (list[PayoutStatus]): The list of available payout status filters
        payout_method (list[PayoutType]): The list of available payout method filters
    """

    connector: list[PayoutConnectors]
    currency: list[Currency]
    status: list[PayoutStatus]
    payout_method: list[PayoutType]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        connector = []
        for connector_item_data in self.connector:
            connector_item = connector_item_data.value
            connector.append(connector_item)

        currency = []
        for currency_item_data in self.currency:
            currency_item = currency_item_data.value
            currency.append(currency_item)

        status = []
        for status_item_data in self.status:
            status_item = status_item_data.value
            status.append(status_item)

        payout_method = []
        for payout_method_item_data in self.payout_method:
            payout_method_item = payout_method_item_data.value
            payout_method.append(payout_method_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "connector": connector,
                "currency": currency,
                "status": status,
                "payout_method": payout_method,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        connector = []
        _connector = d.pop("connector")
        for connector_item_data in _connector:
            connector_item = PayoutConnectors(connector_item_data)

            connector.append(connector_item)

        currency = []
        _currency = d.pop("currency")
        for currency_item_data in _currency:
            currency_item = Currency(currency_item_data)

            currency.append(currency_item)

        status = []
        _status = d.pop("status")
        for status_item_data in _status:
            status_item = PayoutStatus(status_item_data)

            status.append(status_item)

        payout_method = []
        _payout_method = d.pop("payout_method")
        for payout_method_item_data in _payout_method:
            payout_method_item = PayoutType(payout_method_item_data)

            payout_method.append(payout_method_item)

        payout_list_filters = cls(
            connector=connector,
            currency=currency,
            status=status,
            payout_method=payout_method,
        )

        payout_list_filters.additional_properties = d
        return payout_list_filters

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
