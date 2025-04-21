from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.currency import Currency
from ..models.refund_status import RefundStatus
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.amount_filter import AmountFilter


T = TypeVar("T", bound="RefundListRequest")


@_attrs_define
class RefundListRequest:
    """
    Attributes:
        payment_id (Union[None, Unset, str]): The identifier for the payment
        refund_id (Union[None, Unset, str]): The identifier for the refund
        profile_id (Union[None, Unset, str]): The identifier for business profile
        limit (Union[None, Unset, int]): Limit on the number of objects to return
        offset (Union[None, Unset, int]): The starting point within a list of objects
        amount_filter (Union['AmountFilter', None, Unset]):
        connector (Union[None, Unset, list[str]]): The list of connectors to filter refunds list
        merchant_connector_id (Union[None, Unset, list[str]]): The list of merchant connector ids to filter the refunds
            list for selected label
        currency (Union[None, Unset, list[Currency]]): The list of currencies to filter refunds list
        refund_status (Union[None, Unset, list[RefundStatus]]): The list of refund statuses to filter refunds list
    """

    payment_id: Union[None, Unset, str] = UNSET
    refund_id: Union[None, Unset, str] = UNSET
    profile_id: Union[None, Unset, str] = UNSET
    limit: Union[None, Unset, int] = UNSET
    offset: Union[None, Unset, int] = UNSET
    amount_filter: Union["AmountFilter", None, Unset] = UNSET
    connector: Union[None, Unset, list[str]] = UNSET
    merchant_connector_id: Union[None, Unset, list[str]] = UNSET
    currency: Union[None, Unset, list[Currency]] = UNSET
    refund_status: Union[None, Unset, list[RefundStatus]] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.amount_filter import AmountFilter

        payment_id: Union[None, Unset, str]
        if isinstance(self.payment_id, Unset):
            payment_id = UNSET
        else:
            payment_id = self.payment_id

        refund_id: Union[None, Unset, str]
        if isinstance(self.refund_id, Unset):
            refund_id = UNSET
        else:
            refund_id = self.refund_id

        profile_id: Union[None, Unset, str]
        if isinstance(self.profile_id, Unset):
            profile_id = UNSET
        else:
            profile_id = self.profile_id

        limit: Union[None, Unset, int]
        if isinstance(self.limit, Unset):
            limit = UNSET
        else:
            limit = self.limit

        offset: Union[None, Unset, int]
        if isinstance(self.offset, Unset):
            offset = UNSET
        else:
            offset = self.offset

        amount_filter: Union[None, Unset, dict[str, Any]]
        if isinstance(self.amount_filter, Unset):
            amount_filter = UNSET
        elif isinstance(self.amount_filter, AmountFilter):
            amount_filter = self.amount_filter.to_dict()
        else:
            amount_filter = self.amount_filter

        connector: Union[None, Unset, list[str]]
        if isinstance(self.connector, Unset):
            connector = UNSET
        elif isinstance(self.connector, list):
            connector = self.connector

        else:
            connector = self.connector

        merchant_connector_id: Union[None, Unset, list[str]]
        if isinstance(self.merchant_connector_id, Unset):
            merchant_connector_id = UNSET
        elif isinstance(self.merchant_connector_id, list):
            merchant_connector_id = self.merchant_connector_id

        else:
            merchant_connector_id = self.merchant_connector_id

        currency: Union[None, Unset, list[str]]
        if isinstance(self.currency, Unset):
            currency = UNSET
        elif isinstance(self.currency, list):
            currency = []
            for currency_type_0_item_data in self.currency:
                currency_type_0_item = currency_type_0_item_data.value
                currency.append(currency_type_0_item)

        else:
            currency = self.currency

        refund_status: Union[None, Unset, list[str]]
        if isinstance(self.refund_status, Unset):
            refund_status = UNSET
        elif isinstance(self.refund_status, list):
            refund_status = []
            for refund_status_type_0_item_data in self.refund_status:
                refund_status_type_0_item = refund_status_type_0_item_data.value
                refund_status.append(refund_status_type_0_item)

        else:
            refund_status = self.refund_status

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if payment_id is not UNSET:
            field_dict["payment_id"] = payment_id
        if refund_id is not UNSET:
            field_dict["refund_id"] = refund_id
        if profile_id is not UNSET:
            field_dict["profile_id"] = profile_id
        if limit is not UNSET:
            field_dict["limit"] = limit
        if offset is not UNSET:
            field_dict["offset"] = offset
        if amount_filter is not UNSET:
            field_dict["amount_filter"] = amount_filter
        if connector is not UNSET:
            field_dict["connector"] = connector
        if merchant_connector_id is not UNSET:
            field_dict["merchant_connector_id"] = merchant_connector_id
        if currency is not UNSET:
            field_dict["currency"] = currency
        if refund_status is not UNSET:
            field_dict["refund_status"] = refund_status

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.amount_filter import AmountFilter

        d = dict(src_dict)

        def _parse_payment_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_id = _parse_payment_id(d.pop("payment_id", UNSET))

        def _parse_refund_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        refund_id = _parse_refund_id(d.pop("refund_id", UNSET))

        def _parse_profile_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        profile_id = _parse_profile_id(d.pop("profile_id", UNSET))

        def _parse_limit(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        limit = _parse_limit(d.pop("limit", UNSET))

        def _parse_offset(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        offset = _parse_offset(d.pop("offset", UNSET))

        def _parse_amount_filter(data: object) -> Union["AmountFilter", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                amount_filter_type_1 = AmountFilter.from_dict(data)

                return amount_filter_type_1
            except:  # noqa: E722
                pass
            return cast(Union["AmountFilter", None, Unset], data)

        amount_filter = _parse_amount_filter(d.pop("amount_filter", UNSET))

        def _parse_connector(data: object) -> Union[None, Unset, list[str]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                connector_type_0 = cast(list[str], data)

                return connector_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[str]], data)

        connector = _parse_connector(d.pop("connector", UNSET))

        def _parse_merchant_connector_id(data: object) -> Union[None, Unset, list[str]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                merchant_connector_id_type_0 = cast(list[str], data)

                return merchant_connector_id_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[str]], data)

        merchant_connector_id = _parse_merchant_connector_id(d.pop("merchant_connector_id", UNSET))

        def _parse_currency(data: object) -> Union[None, Unset, list[Currency]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                currency_type_0 = []
                _currency_type_0 = data
                for currency_type_0_item_data in _currency_type_0:
                    currency_type_0_item = Currency(currency_type_0_item_data)

                    currency_type_0.append(currency_type_0_item)

                return currency_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[Currency]], data)

        currency = _parse_currency(d.pop("currency", UNSET))

        def _parse_refund_status(data: object) -> Union[None, Unset, list[RefundStatus]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                refund_status_type_0 = []
                _refund_status_type_0 = data
                for refund_status_type_0_item_data in _refund_status_type_0:
                    refund_status_type_0_item = RefundStatus(refund_status_type_0_item_data)

                    refund_status_type_0.append(refund_status_type_0_item)

                return refund_status_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[RefundStatus]], data)

        refund_status = _parse_refund_status(d.pop("refund_status", UNSET))

        refund_list_request = cls(
            payment_id=payment_id,
            refund_id=refund_id,
            profile_id=profile_id,
            limit=limit,
            offset=offset,
            amount_filter=amount_filter,
            connector=connector,
            merchant_connector_id=merchant_connector_id,
            currency=currency,
            refund_status=refund_status,
        )

        refund_list_request.additional_properties = d
        return refund_list_request

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
