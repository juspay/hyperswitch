from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.authentication_connectors import AuthenticationConnectors
from ..types import UNSET, Unset

T = TypeVar("T", bound="AuthenticationConnectorDetails")


@_attrs_define
class AuthenticationConnectorDetails:
    """
    Attributes:
        authentication_connectors (list[AuthenticationConnectors]): List of authentication connectors
        three_ds_requestor_url (str): URL of the (customer service) website that will be shown to the shopper in case of
            technical errors during the 3D Secure 2 process.
        three_ds_requestor_app_url (Union[None, Unset, str]): Merchant app declaring their URL within the CReq message
            so that the Authentication app can call the Merchant app after OOB authentication has occurred.
    """

    authentication_connectors: list[AuthenticationConnectors]
    three_ds_requestor_url: str
    three_ds_requestor_app_url: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        authentication_connectors = []
        for authentication_connectors_item_data in self.authentication_connectors:
            authentication_connectors_item = authentication_connectors_item_data.value
            authentication_connectors.append(authentication_connectors_item)

        three_ds_requestor_url = self.three_ds_requestor_url

        three_ds_requestor_app_url: Union[None, Unset, str]
        if isinstance(self.three_ds_requestor_app_url, Unset):
            three_ds_requestor_app_url = UNSET
        else:
            three_ds_requestor_app_url = self.three_ds_requestor_app_url

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "authentication_connectors": authentication_connectors,
                "three_ds_requestor_url": three_ds_requestor_url,
            }
        )
        if three_ds_requestor_app_url is not UNSET:
            field_dict["three_ds_requestor_app_url"] = three_ds_requestor_app_url

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        authentication_connectors = []
        _authentication_connectors = d.pop("authentication_connectors")
        for authentication_connectors_item_data in _authentication_connectors:
            authentication_connectors_item = AuthenticationConnectors(authentication_connectors_item_data)

            authentication_connectors.append(authentication_connectors_item)

        three_ds_requestor_url = d.pop("three_ds_requestor_url")

        def _parse_three_ds_requestor_app_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        three_ds_requestor_app_url = _parse_three_ds_requestor_app_url(d.pop("three_ds_requestor_app_url", UNSET))

        authentication_connector_details = cls(
            authentication_connectors=authentication_connectors,
            three_ds_requestor_url=three_ds_requestor_url,
            three_ds_requestor_app_url=three_ds_requestor_app_url,
        )

        authentication_connector_details.additional_properties = d
        return authentication_connector_details

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
