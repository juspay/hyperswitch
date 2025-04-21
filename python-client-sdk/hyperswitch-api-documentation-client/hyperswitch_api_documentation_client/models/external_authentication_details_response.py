from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.authentication_status import AuthenticationStatus
from ..models.decoupled_authentication_type import DecoupledAuthenticationType
from ..types import UNSET, Unset

T = TypeVar("T", bound="ExternalAuthenticationDetailsResponse")


@_attrs_define
class ExternalAuthenticationDetailsResponse:
    """Details of external authentication

    Attributes:
        status (AuthenticationStatus):
        authentication_flow (Union[DecoupledAuthenticationType, None, Unset]):
        electronic_commerce_indicator (Union[None, Unset, str]): Electronic Commerce Indicator (eci)
        ds_transaction_id (Union[None, Unset, str]): DS Transaction ID
        version (Union[None, Unset, str]): Message Version
        error_code (Union[None, Unset, str]): Error Code
        error_message (Union[None, Unset, str]): Error Message
    """

    status: AuthenticationStatus
    authentication_flow: Union[DecoupledAuthenticationType, None, Unset] = UNSET
    electronic_commerce_indicator: Union[None, Unset, str] = UNSET
    ds_transaction_id: Union[None, Unset, str] = UNSET
    version: Union[None, Unset, str] = UNSET
    error_code: Union[None, Unset, str] = UNSET
    error_message: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        status = self.status.value

        authentication_flow: Union[None, Unset, str]
        if isinstance(self.authentication_flow, Unset):
            authentication_flow = UNSET
        elif isinstance(self.authentication_flow, DecoupledAuthenticationType):
            authentication_flow = self.authentication_flow.value
        else:
            authentication_flow = self.authentication_flow

        electronic_commerce_indicator: Union[None, Unset, str]
        if isinstance(self.electronic_commerce_indicator, Unset):
            electronic_commerce_indicator = UNSET
        else:
            electronic_commerce_indicator = self.electronic_commerce_indicator

        ds_transaction_id: Union[None, Unset, str]
        if isinstance(self.ds_transaction_id, Unset):
            ds_transaction_id = UNSET
        else:
            ds_transaction_id = self.ds_transaction_id

        version: Union[None, Unset, str]
        if isinstance(self.version, Unset):
            version = UNSET
        else:
            version = self.version

        error_code: Union[None, Unset, str]
        if isinstance(self.error_code, Unset):
            error_code = UNSET
        else:
            error_code = self.error_code

        error_message: Union[None, Unset, str]
        if isinstance(self.error_message, Unset):
            error_message = UNSET
        else:
            error_message = self.error_message

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "status": status,
            }
        )
        if authentication_flow is not UNSET:
            field_dict["authentication_flow"] = authentication_flow
        if electronic_commerce_indicator is not UNSET:
            field_dict["electronic_commerce_indicator"] = electronic_commerce_indicator
        if ds_transaction_id is not UNSET:
            field_dict["ds_transaction_id"] = ds_transaction_id
        if version is not UNSET:
            field_dict["version"] = version
        if error_code is not UNSET:
            field_dict["error_code"] = error_code
        if error_message is not UNSET:
            field_dict["error_message"] = error_message

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        status = AuthenticationStatus(d.pop("status"))

        def _parse_authentication_flow(data: object) -> Union[DecoupledAuthenticationType, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                authentication_flow_type_1 = DecoupledAuthenticationType(data)

                return authentication_flow_type_1
            except:  # noqa: E722
                pass
            return cast(Union[DecoupledAuthenticationType, None, Unset], data)

        authentication_flow = _parse_authentication_flow(d.pop("authentication_flow", UNSET))

        def _parse_electronic_commerce_indicator(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        electronic_commerce_indicator = _parse_electronic_commerce_indicator(
            d.pop("electronic_commerce_indicator", UNSET)
        )

        def _parse_ds_transaction_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        ds_transaction_id = _parse_ds_transaction_id(d.pop("ds_transaction_id", UNSET))

        def _parse_version(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        version = _parse_version(d.pop("version", UNSET))

        def _parse_error_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_code = _parse_error_code(d.pop("error_code", UNSET))

        def _parse_error_message(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_message = _parse_error_message(d.pop("error_message", UNSET))

        external_authentication_details_response = cls(
            status=status,
            authentication_flow=authentication_flow,
            electronic_commerce_indicator=electronic_commerce_indicator,
            ds_transaction_id=ds_transaction_id,
            version=version,
            error_code=error_code,
            error_message=error_message,
        )

        external_authentication_details_response.additional_properties = d
        return external_authentication_details_response

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
