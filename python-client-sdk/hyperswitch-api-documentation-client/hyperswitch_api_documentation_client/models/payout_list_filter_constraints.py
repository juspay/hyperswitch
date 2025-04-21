from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.currency import Currency
from ..models.payout_connectors import PayoutConnectors
from ..models.payout_entity_type import PayoutEntityType
from ..models.payout_status import PayoutStatus
from ..models.payout_type import PayoutType
from ..types import UNSET, Unset

T = TypeVar("T", bound="PayoutListFilterConstraints")


@_attrs_define
class PayoutListFilterConstraints:
    """
    Attributes:
        currency (Currency): The three letter ISO currency code in uppercase. Eg: 'USD' for the United States Dollar.
        entity_type (PayoutEntityType): Type of entity to whom the payout is being carried out to, select from the given
            list of options
        payout_id (Union[None, Unset, str]): The identifier for payout Example: 187282ab-40ef-47a9-9206-5099ba31e432.
        profile_id (Union[None, Unset, str]): The identifier for business profile
        customer_id (Union[None, Unset, str]): The identifier for customer Example: cus_y3oqhf46pyzuxjbcn2giaqnb44.
        limit (Union[Unset, int]): The limit on the number of objects. The default limit is 10 and max limit is 20
        offset (Union[None, Unset, int]): The starting point within a list of objects
        connector (Union[None, Unset, list[PayoutConnectors]]): The list of connectors to filter payouts list Example:
            ['wise', 'adyen'].
        status (Union[None, Unset, list[PayoutStatus]]): The list of payout status to filter payouts list Example:
            ['pending', 'failed'].
        payout_method (Union[None, Unset, list[PayoutType]]): The list of payout methods to filter payouts list Example:
            ['bank', 'card'].
    """

    currency: Currency
    entity_type: PayoutEntityType
    payout_id: Union[None, Unset, str] = UNSET
    profile_id: Union[None, Unset, str] = UNSET
    customer_id: Union[None, Unset, str] = UNSET
    limit: Union[Unset, int] = UNSET
    offset: Union[None, Unset, int] = UNSET
    connector: Union[None, Unset, list[PayoutConnectors]] = UNSET
    status: Union[None, Unset, list[PayoutStatus]] = UNSET
    payout_method: Union[None, Unset, list[PayoutType]] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        currency = self.currency.value

        entity_type = self.entity_type.value

        payout_id: Union[None, Unset, str]
        if isinstance(self.payout_id, Unset):
            payout_id = UNSET
        else:
            payout_id = self.payout_id

        profile_id: Union[None, Unset, str]
        if isinstance(self.profile_id, Unset):
            profile_id = UNSET
        else:
            profile_id = self.profile_id

        customer_id: Union[None, Unset, str]
        if isinstance(self.customer_id, Unset):
            customer_id = UNSET
        else:
            customer_id = self.customer_id

        limit = self.limit

        offset: Union[None, Unset, int]
        if isinstance(self.offset, Unset):
            offset = UNSET
        else:
            offset = self.offset

        connector: Union[None, Unset, list[str]]
        if isinstance(self.connector, Unset):
            connector = UNSET
        elif isinstance(self.connector, list):
            connector = []
            for connector_type_0_item_data in self.connector:
                connector_type_0_item = connector_type_0_item_data.value
                connector.append(connector_type_0_item)

        else:
            connector = self.connector

        status: Union[None, Unset, list[str]]
        if isinstance(self.status, Unset):
            status = UNSET
        elif isinstance(self.status, list):
            status = []
            for status_type_0_item_data in self.status:
                status_type_0_item = status_type_0_item_data.value
                status.append(status_type_0_item)

        else:
            status = self.status

        payout_method: Union[None, Unset, list[str]]
        if isinstance(self.payout_method, Unset):
            payout_method = UNSET
        elif isinstance(self.payout_method, list):
            payout_method = []
            for payout_method_type_0_item_data in self.payout_method:
                payout_method_type_0_item = payout_method_type_0_item_data.value
                payout_method.append(payout_method_type_0_item)

        else:
            payout_method = self.payout_method

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "currency": currency,
                "entity_type": entity_type,
            }
        )
        if payout_id is not UNSET:
            field_dict["payout_id"] = payout_id
        if profile_id is not UNSET:
            field_dict["profile_id"] = profile_id
        if customer_id is not UNSET:
            field_dict["customer_id"] = customer_id
        if limit is not UNSET:
            field_dict["limit"] = limit
        if offset is not UNSET:
            field_dict["offset"] = offset
        if connector is not UNSET:
            field_dict["connector"] = connector
        if status is not UNSET:
            field_dict["status"] = status
        if payout_method is not UNSET:
            field_dict["payout_method"] = payout_method

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        currency = Currency(d.pop("currency"))

        entity_type = PayoutEntityType(d.pop("entity_type"))

        def _parse_payout_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payout_id = _parse_payout_id(d.pop("payout_id", UNSET))

        def _parse_profile_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        profile_id = _parse_profile_id(d.pop("profile_id", UNSET))

        def _parse_customer_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        customer_id = _parse_customer_id(d.pop("customer_id", UNSET))

        limit = d.pop("limit", UNSET)

        def _parse_offset(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        offset = _parse_offset(d.pop("offset", UNSET))

        def _parse_connector(data: object) -> Union[None, Unset, list[PayoutConnectors]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                connector_type_0 = []
                _connector_type_0 = data
                for connector_type_0_item_data in _connector_type_0:
                    connector_type_0_item = PayoutConnectors(connector_type_0_item_data)

                    connector_type_0.append(connector_type_0_item)

                return connector_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[PayoutConnectors]], data)

        connector = _parse_connector(d.pop("connector", UNSET))

        def _parse_status(data: object) -> Union[None, Unset, list[PayoutStatus]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                status_type_0 = []
                _status_type_0 = data
                for status_type_0_item_data in _status_type_0:
                    status_type_0_item = PayoutStatus(status_type_0_item_data)

                    status_type_0.append(status_type_0_item)

                return status_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[PayoutStatus]], data)

        status = _parse_status(d.pop("status", UNSET))

        def _parse_payout_method(data: object) -> Union[None, Unset, list[PayoutType]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                payout_method_type_0 = []
                _payout_method_type_0 = data
                for payout_method_type_0_item_data in _payout_method_type_0:
                    payout_method_type_0_item = PayoutType(payout_method_type_0_item_data)

                    payout_method_type_0.append(payout_method_type_0_item)

                return payout_method_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[PayoutType]], data)

        payout_method = _parse_payout_method(d.pop("payout_method", UNSET))

        payout_list_filter_constraints = cls(
            currency=currency,
            entity_type=entity_type,
            payout_id=payout_id,
            profile_id=profile_id,
            customer_id=customer_id,
            limit=limit,
            offset=offset,
            connector=connector,
            status=status,
            payout_method=payout_method,
        )

        payout_list_filter_constraints.additional_properties = d
        return payout_list_filter_constraints

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
