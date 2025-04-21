import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from dateutil.parser import isoparse

from ..models.api_key_expiration_type_0 import ApiKeyExpirationType0
from ..types import UNSET, Unset

T = TypeVar("T", bound="UpdateApiKeyRequest")


@_attrs_define
class UpdateApiKeyRequest:
    """The request body for updating an API Key.

    Attributes:
        name (Union[None, Unset, str]): A unique name for the API Key to help you identify it. Example: Sandbox
            integration key.
        description (Union[None, Unset, str]): A description to provide more context about the API Key. Example: Key
            used by our developers to integrate with the sandbox environment.
        expiration (Union[ApiKeyExpirationType0, None, Unset, datetime.datetime]):
    """

    name: Union[None, Unset, str] = UNSET
    description: Union[None, Unset, str] = UNSET
    expiration: Union[ApiKeyExpirationType0, None, Unset, datetime.datetime] = UNSET

    def to_dict(self) -> dict[str, Any]:
        name: Union[None, Unset, str]
        if isinstance(self.name, Unset):
            name = UNSET
        else:
            name = self.name

        description: Union[None, Unset, str]
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        expiration: Union[None, Unset, str]
        if isinstance(self.expiration, Unset):
            expiration = UNSET
        elif isinstance(self.expiration, ApiKeyExpirationType0):
            expiration = self.expiration.value
        elif isinstance(self.expiration, datetime.datetime):
            expiration = self.expiration.isoformat()
        else:
            expiration = self.expiration

        field_dict: dict[str, Any] = {}
        field_dict.update({})
        if name is not UNSET:
            field_dict["name"] = name
        if description is not UNSET:
            field_dict["description"] = description
        if expiration is not UNSET:
            field_dict["expiration"] = expiration

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        name = _parse_name(d.pop("name", UNSET))

        def _parse_description(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        description = _parse_description(d.pop("description", UNSET))

        def _parse_expiration(data: object) -> Union[ApiKeyExpirationType0, None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_api_key_expiration_type_0 = ApiKeyExpirationType0(data)

                return componentsschemas_api_key_expiration_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_api_key_expiration_type_1 = isoparse(data)

                return componentsschemas_api_key_expiration_type_1
            except:  # noqa: E722
                pass
            return cast(Union[ApiKeyExpirationType0, None, Unset, datetime.datetime], data)

        expiration = _parse_expiration(d.pop("expiration", UNSET))

        update_api_key_request = cls(
            name=name,
            description=description,
            expiration=expiration,
        )

        return update_api_key_request
