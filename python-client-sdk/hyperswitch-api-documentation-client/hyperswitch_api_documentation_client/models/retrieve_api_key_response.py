import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..models.api_key_expiration_type_0 import ApiKeyExpirationType0
from ..types import UNSET, Unset

T = TypeVar("T", bound="RetrieveApiKeyResponse")


@_attrs_define
class RetrieveApiKeyResponse:
    """The response body for retrieving an API Key.

    Attributes:
        key_id (str): The identifier for the API Key. Example: 5hEEqkgJUyuxgSKGArHA4mWSnX.
        merchant_id (str): The identifier for the Merchant Account. Example: y3oqhf46pyzuxjbcn2giaqnb44.
        name (str): The unique name for the API Key to help you identify it. Example: Sandbox integration key.
        prefix (str): The first few characters of the plaintext API Key to help you identify it.
        created (datetime.datetime): The time at which the API Key was created. Example: 2022-09-10T10:11:12Z.
        expiration (Union[ApiKeyExpirationType0, datetime.datetime]):
        description (Union[None, Unset, str]): The description to provide more context about the API Key. Example: Key
            used by our developers to integrate with the sandbox environment.
    """

    key_id: str
    merchant_id: str
    name: str
    prefix: str
    created: datetime.datetime
    expiration: Union[ApiKeyExpirationType0, datetime.datetime]
    description: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        key_id = self.key_id

        merchant_id = self.merchant_id

        name = self.name

        prefix = self.prefix

        created = self.created.isoformat()

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
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "key_id": key_id,
                "merchant_id": merchant_id,
                "name": name,
                "prefix": prefix,
                "created": created,
                "expiration": expiration,
            }
        )
        if description is not UNSET:
            field_dict["description"] = description

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        key_id = d.pop("key_id")

        merchant_id = d.pop("merchant_id")

        name = d.pop("name")

        prefix = d.pop("prefix")

        created = isoparse(d.pop("created"))

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

        retrieve_api_key_response = cls(
            key_id=key_id,
            merchant_id=merchant_id,
            name=name,
            prefix=prefix,
            created=created,
            expiration=expiration,
            description=description,
        )

        retrieve_api_key_response.additional_properties = d
        return retrieve_api_key_response

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
