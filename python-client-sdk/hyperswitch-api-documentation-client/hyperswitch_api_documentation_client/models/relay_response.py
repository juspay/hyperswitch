from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.relay_status import RelayStatus
from ..models.relay_type import RelayType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.relay_data_type_0 import RelayDataType0
    from ..models.relay_error import RelayError


T = TypeVar("T", bound="RelayResponse")


@_attrs_define
class RelayResponse:
    """
    Attributes:
        id (str): The unique identifier for the Relay Example: relay_mbabizu24mvu3mela5njyhpit4.
        status (RelayStatus):
        connector_resource_id (str): The identifier that is associated to a resource at the connector reference to which
            the relay request is being made Example: pi_3MKEivSFNglxLpam0ZaL98q9.
        connector_id (str): Identifier of the connector ( merchant connector account ) which was chosen to make the
            payment Example: mca_5apGeP94tMts6rg3U3kR.
        profile_id (str): The business profile that is associated with this relay request. Example:
            pro_abcdefghijklmnopqrstuvwxyz.
        type_ (RelayType):
        error (Union['RelayError', None, Unset]):
        connector_reference_id (Union[None, Unset, str]): The identifier that is associated to a resource at the
            connector to which the relay request is being made Example: re_3QY4TnEOqOywnAIx1Mm1p7GQ.
        data (Union['RelayDataType0', None, Unset]):
    """

    id: str
    status: RelayStatus
    connector_resource_id: str
    connector_id: str
    profile_id: str
    type_: RelayType
    error: Union["RelayError", None, Unset] = UNSET
    connector_reference_id: Union[None, Unset, str] = UNSET
    data: Union["RelayDataType0", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.relay_data_type_0 import RelayDataType0
        from ..models.relay_error import RelayError

        id = self.id

        status = self.status.value

        connector_resource_id = self.connector_resource_id

        connector_id = self.connector_id

        profile_id = self.profile_id

        type_ = self.type_.value

        error: Union[None, Unset, dict[str, Any]]
        if isinstance(self.error, Unset):
            error = UNSET
        elif isinstance(self.error, RelayError):
            error = self.error.to_dict()
        else:
            error = self.error

        connector_reference_id: Union[None, Unset, str]
        if isinstance(self.connector_reference_id, Unset):
            connector_reference_id = UNSET
        else:
            connector_reference_id = self.connector_reference_id

        data: Union[None, Unset, dict[str, Any]]
        if isinstance(self.data, Unset):
            data = UNSET
        elif isinstance(self.data, RelayDataType0):
            data = self.data.to_dict()
        else:
            data = self.data

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "status": status,
                "connector_resource_id": connector_resource_id,
                "connector_id": connector_id,
                "profile_id": profile_id,
                "type": type_,
            }
        )
        if error is not UNSET:
            field_dict["error"] = error
        if connector_reference_id is not UNSET:
            field_dict["connector_reference_id"] = connector_reference_id
        if data is not UNSET:
            field_dict["data"] = data

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.relay_data_type_0 import RelayDataType0
        from ..models.relay_error import RelayError

        d = dict(src_dict)
        id = d.pop("id")

        status = RelayStatus(d.pop("status"))

        connector_resource_id = d.pop("connector_resource_id")

        connector_id = d.pop("connector_id")

        profile_id = d.pop("profile_id")

        type_ = RelayType(d.pop("type"))

        def _parse_error(data: object) -> Union["RelayError", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                error_type_1 = RelayError.from_dict(data)

                return error_type_1
            except:  # noqa: E722
                pass
            return cast(Union["RelayError", None, Unset], data)

        error = _parse_error(d.pop("error", UNSET))

        def _parse_connector_reference_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_reference_id = _parse_connector_reference_id(d.pop("connector_reference_id", UNSET))

        def _parse_data(data: object) -> Union["RelayDataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_relay_data_type_0 = RelayDataType0.from_dict(data)

                return componentsschemas_relay_data_type_0
            except:  # noqa: E722
                pass
            return cast(Union["RelayDataType0", None, Unset], data)

        data = _parse_data(d.pop("data", UNSET))

        relay_response = cls(
            id=id,
            status=status,
            connector_resource_id=connector_resource_id,
            connector_id=connector_id,
            profile_id=profile_id,
            type_=type_,
            error=error,
            connector_reference_id=connector_reference_id,
            data=data,
        )

        relay_response.additional_properties = d
        return relay_response

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
