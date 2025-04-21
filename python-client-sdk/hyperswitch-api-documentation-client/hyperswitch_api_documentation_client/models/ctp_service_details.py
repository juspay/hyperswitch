from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.ctp_service_provider import CtpServiceProvider
from ..types import UNSET, Unset

T = TypeVar("T", bound="CtpServiceDetails")


@_attrs_define
class CtpServiceDetails:
    """
    Attributes:
        merchant_transaction_id (Union[None, Unset, str]): merchant transaction id
        correlation_id (Union[None, Unset, str]): network transaction correlation id
        x_src_flow_id (Union[None, Unset, str]): session transaction flow id
        provider (Union[CtpServiceProvider, None, Unset]):
        encypted_payload (Union[None, Unset, str]): Encrypted payload
    """

    merchant_transaction_id: Union[None, Unset, str] = UNSET
    correlation_id: Union[None, Unset, str] = UNSET
    x_src_flow_id: Union[None, Unset, str] = UNSET
    provider: Union[CtpServiceProvider, None, Unset] = UNSET
    encypted_payload: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        merchant_transaction_id: Union[None, Unset, str]
        if isinstance(self.merchant_transaction_id, Unset):
            merchant_transaction_id = UNSET
        else:
            merchant_transaction_id = self.merchant_transaction_id

        correlation_id: Union[None, Unset, str]
        if isinstance(self.correlation_id, Unset):
            correlation_id = UNSET
        else:
            correlation_id = self.correlation_id

        x_src_flow_id: Union[None, Unset, str]
        if isinstance(self.x_src_flow_id, Unset):
            x_src_flow_id = UNSET
        else:
            x_src_flow_id = self.x_src_flow_id

        provider: Union[None, Unset, str]
        if isinstance(self.provider, Unset):
            provider = UNSET
        elif isinstance(self.provider, CtpServiceProvider):
            provider = self.provider.value
        else:
            provider = self.provider

        encypted_payload: Union[None, Unset, str]
        if isinstance(self.encypted_payload, Unset):
            encypted_payload = UNSET
        else:
            encypted_payload = self.encypted_payload

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if merchant_transaction_id is not UNSET:
            field_dict["merchant_transaction_id"] = merchant_transaction_id
        if correlation_id is not UNSET:
            field_dict["correlation_id"] = correlation_id
        if x_src_flow_id is not UNSET:
            field_dict["x_src_flow_id"] = x_src_flow_id
        if provider is not UNSET:
            field_dict["provider"] = provider
        if encypted_payload is not UNSET:
            field_dict["encypted_payload"] = encypted_payload

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_merchant_transaction_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_transaction_id = _parse_merchant_transaction_id(d.pop("merchant_transaction_id", UNSET))

        def _parse_correlation_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        correlation_id = _parse_correlation_id(d.pop("correlation_id", UNSET))

        def _parse_x_src_flow_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        x_src_flow_id = _parse_x_src_flow_id(d.pop("x_src_flow_id", UNSET))

        def _parse_provider(data: object) -> Union[CtpServiceProvider, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                provider_type_1 = CtpServiceProvider(data)

                return provider_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CtpServiceProvider, None, Unset], data)

        provider = _parse_provider(d.pop("provider", UNSET))

        def _parse_encypted_payload(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        encypted_payload = _parse_encypted_payload(d.pop("encypted_payload", UNSET))

        ctp_service_details = cls(
            merchant_transaction_id=merchant_transaction_id,
            correlation_id=correlation_id,
            x_src_flow_id=x_src_flow_id,
            provider=provider,
            encypted_payload=encypted_payload,
        )

        ctp_service_details.additional_properties = d
        return ctp_service_details

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
