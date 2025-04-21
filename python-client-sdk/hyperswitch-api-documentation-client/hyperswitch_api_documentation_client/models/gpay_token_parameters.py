from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="GpayTokenParameters")


@_attrs_define
class GpayTokenParameters:
    """
    Attributes:
        gateway (Union[None, Unset, str]): The name of the connector
        gateway_merchant_id (Union[None, Unset, str]): The merchant ID registered in the connector associated
        stripeversion (Union[None, Unset, str]):
        stripepublishable_key (Union[None, Unset, str]):
        protocol_version (Union[None, Unset, str]): The protocol version for encryption
        public_key (Union[None, Unset, str]): The public key provided by the merchant
    """

    gateway: Union[None, Unset, str] = UNSET
    gateway_merchant_id: Union[None, Unset, str] = UNSET
    stripeversion: Union[None, Unset, str] = UNSET
    stripepublishable_key: Union[None, Unset, str] = UNSET
    protocol_version: Union[None, Unset, str] = UNSET
    public_key: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        gateway: Union[None, Unset, str]
        if isinstance(self.gateway, Unset):
            gateway = UNSET
        else:
            gateway = self.gateway

        gateway_merchant_id: Union[None, Unset, str]
        if isinstance(self.gateway_merchant_id, Unset):
            gateway_merchant_id = UNSET
        else:
            gateway_merchant_id = self.gateway_merchant_id

        stripeversion: Union[None, Unset, str]
        if isinstance(self.stripeversion, Unset):
            stripeversion = UNSET
        else:
            stripeversion = self.stripeversion

        stripepublishable_key: Union[None, Unset, str]
        if isinstance(self.stripepublishable_key, Unset):
            stripepublishable_key = UNSET
        else:
            stripepublishable_key = self.stripepublishable_key

        protocol_version: Union[None, Unset, str]
        if isinstance(self.protocol_version, Unset):
            protocol_version = UNSET
        else:
            protocol_version = self.protocol_version

        public_key: Union[None, Unset, str]
        if isinstance(self.public_key, Unset):
            public_key = UNSET
        else:
            public_key = self.public_key

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if gateway is not UNSET:
            field_dict["gateway"] = gateway
        if gateway_merchant_id is not UNSET:
            field_dict["gateway_merchant_id"] = gateway_merchant_id
        if stripeversion is not UNSET:
            field_dict["stripe:version"] = stripeversion
        if stripepublishable_key is not UNSET:
            field_dict["stripe:publishableKey"] = stripepublishable_key
        if protocol_version is not UNSET:
            field_dict["protocol_version"] = protocol_version
        if public_key is not UNSET:
            field_dict["public_key"] = public_key

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_gateway(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        gateway = _parse_gateway(d.pop("gateway", UNSET))

        def _parse_gateway_merchant_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        gateway_merchant_id = _parse_gateway_merchant_id(d.pop("gateway_merchant_id", UNSET))

        def _parse_stripeversion(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        stripeversion = _parse_stripeversion(d.pop("stripe:version", UNSET))

        def _parse_stripepublishable_key(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        stripepublishable_key = _parse_stripepublishable_key(d.pop("stripe:publishableKey", UNSET))

        def _parse_protocol_version(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        protocol_version = _parse_protocol_version(d.pop("protocol_version", UNSET))

        def _parse_public_key(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        public_key = _parse_public_key(d.pop("public_key", UNSET))

        gpay_token_parameters = cls(
            gateway=gateway,
            gateway_merchant_id=gateway_merchant_id,
            stripeversion=stripeversion,
            stripepublishable_key=stripepublishable_key,
            protocol_version=protocol_version,
            public_key=public_key,
        )

        gpay_token_parameters.additional_properties = d
        return gpay_token_parameters

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
