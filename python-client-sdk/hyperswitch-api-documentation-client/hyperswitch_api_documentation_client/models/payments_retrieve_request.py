from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap


T = TypeVar("T", bound="PaymentsRetrieveRequest")


@_attrs_define
class PaymentsRetrieveRequest:
    """
    Attributes:
        resource_id (str): The type of ID (ex: payment intent id, payment attempt id or connector txn id)
        force_sync (bool): Decider to enable or disable the connector call for retrieve request
        merchant_id (Union[None, Unset, str]): The identifier for the Merchant Account.
        param (Union[None, Unset, str]): The parameters passed to a retrieve request
        connector (Union[None, Unset, str]): The name of the connector
        merchant_connector_details (Union['MerchantConnectorDetailsWrap', None, Unset]):
        client_secret (Union[None, Unset, str]): This is a token which expires after 15 minutes, used from the client to
            authenticate and create sessions from the SDK
        expand_captures (Union[None, Unset, bool]): If enabled provides list of captures linked to latest attempt
        expand_attempts (Union[None, Unset, bool]): If enabled provides list of attempts linked to payment intent
    """

    resource_id: str
    force_sync: bool
    merchant_id: Union[None, Unset, str] = UNSET
    param: Union[None, Unset, str] = UNSET
    connector: Union[None, Unset, str] = UNSET
    merchant_connector_details: Union["MerchantConnectorDetailsWrap", None, Unset] = UNSET
    client_secret: Union[None, Unset, str] = UNSET
    expand_captures: Union[None, Unset, bool] = UNSET
    expand_attempts: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap

        resource_id = self.resource_id

        force_sync = self.force_sync

        merchant_id: Union[None, Unset, str]
        if isinstance(self.merchant_id, Unset):
            merchant_id = UNSET
        else:
            merchant_id = self.merchant_id

        param: Union[None, Unset, str]
        if isinstance(self.param, Unset):
            param = UNSET
        else:
            param = self.param

        connector: Union[None, Unset, str]
        if isinstance(self.connector, Unset):
            connector = UNSET
        else:
            connector = self.connector

        merchant_connector_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.merchant_connector_details, Unset):
            merchant_connector_details = UNSET
        elif isinstance(self.merchant_connector_details, MerchantConnectorDetailsWrap):
            merchant_connector_details = self.merchant_connector_details.to_dict()
        else:
            merchant_connector_details = self.merchant_connector_details

        client_secret: Union[None, Unset, str]
        if isinstance(self.client_secret, Unset):
            client_secret = UNSET
        else:
            client_secret = self.client_secret

        expand_captures: Union[None, Unset, bool]
        if isinstance(self.expand_captures, Unset):
            expand_captures = UNSET
        else:
            expand_captures = self.expand_captures

        expand_attempts: Union[None, Unset, bool]
        if isinstance(self.expand_attempts, Unset):
            expand_attempts = UNSET
        else:
            expand_attempts = self.expand_attempts

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "resource_id": resource_id,
                "force_sync": force_sync,
            }
        )
        if merchant_id is not UNSET:
            field_dict["merchant_id"] = merchant_id
        if param is not UNSET:
            field_dict["param"] = param
        if connector is not UNSET:
            field_dict["connector"] = connector
        if merchant_connector_details is not UNSET:
            field_dict["merchant_connector_details"] = merchant_connector_details
        if client_secret is not UNSET:
            field_dict["client_secret"] = client_secret
        if expand_captures is not UNSET:
            field_dict["expand_captures"] = expand_captures
        if expand_attempts is not UNSET:
            field_dict["expand_attempts"] = expand_attempts

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap

        d = dict(src_dict)
        resource_id = d.pop("resource_id")

        force_sync = d.pop("force_sync")

        def _parse_merchant_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_id = _parse_merchant_id(d.pop("merchant_id", UNSET))

        def _parse_param(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        param = _parse_param(d.pop("param", UNSET))

        def _parse_connector(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector = _parse_connector(d.pop("connector", UNSET))

        def _parse_merchant_connector_details(data: object) -> Union["MerchantConnectorDetailsWrap", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                merchant_connector_details_type_1 = MerchantConnectorDetailsWrap.from_dict(data)

                return merchant_connector_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorDetailsWrap", None, Unset], data)

        merchant_connector_details = _parse_merchant_connector_details(d.pop("merchant_connector_details", UNSET))

        def _parse_client_secret(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        client_secret = _parse_client_secret(d.pop("client_secret", UNSET))

        def _parse_expand_captures(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        expand_captures = _parse_expand_captures(d.pop("expand_captures", UNSET))

        def _parse_expand_attempts(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        expand_attempts = _parse_expand_attempts(d.pop("expand_attempts", UNSET))

        payments_retrieve_request = cls(
            resource_id=resource_id,
            force_sync=force_sync,
            merchant_id=merchant_id,
            param=param,
            connector=connector,
            merchant_connector_details=merchant_connector_details,
            client_secret=client_secret,
            expand_captures=expand_captures,
            expand_attempts=expand_attempts,
        )

        payments_retrieve_request.additional_properties = d
        return payments_retrieve_request

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
