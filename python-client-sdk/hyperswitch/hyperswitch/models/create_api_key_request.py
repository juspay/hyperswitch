import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from dateutil.parser import isoparse

from ..models.api_key_expiration_type_0 import ApiKeyExpirationType0
from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateApiKeyRequest")


@_attrs_define
class CreateApiKeyRequest:
    """The request body for creating an API Key.

    Attributes:
        name (str): A unique name for the API Key to help you identify it. Example: Sandbox integration key.
        expiration (Union[ApiKeyExpirationType0, datetime.datetime]):
        description (Union[None, Unset, str]): A description to provide more context about the API Key. Example: Key
            used by our developers to integrate with the sandbox environment.
    """

    name: str
    expiration: Union[ApiKeyExpirationType0, datetime.datetime]
    description: Union[None, Unset, str] = UNSET

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        expiration: str
        if isinstance(self.expiration, ApiKeyExpirationType0):
            expiration = self.expiration.value
        else:
            expiration = self.expiration.isoformat()

        description: Union[None, Unset, str]
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "name": name,
                "expiration": expiration,
            }
        )
        if description is not UNSET:
            field_dict["description"] = description

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        name = d.pop("name")

        def _parse_expiration(data: object) -> Union[ApiKeyExpirationType0, datetime.datetime]:
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_api_key_expiration_type_0 = ApiKeyExpirationType0(data)

                return componentsschemas_api_key_expiration_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, str):
                raise TypeError()
            componentsschemas_api_key_expiration_type_1 = isoparse(data)

            return componentsschemas_api_key_expiration_type_1

        expiration = _parse_expiration(d.pop("expiration"))

        def _parse_description(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        description = _parse_description(d.pop("description", UNSET))

        create_api_key_request = cls(
            name=name,
            expiration=expiration,
            description=description,
        )

        return create_api_key_request
