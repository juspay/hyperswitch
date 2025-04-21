import datetime
from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.organization_response_metadata_type_0 import OrganizationResponseMetadataType0
    from ..models.organization_response_organization_details_type_0 import OrganizationResponseOrganizationDetailsType0


T = TypeVar("T", bound="OrganizationResponse")


@_attrs_define
class OrganizationResponse:
    """
    Attributes:
        organization_id (str): The unique identifier for the Organization Example: org_q98uSGAYbjEwqs0mJwnz.
        modified_at (datetime.datetime):
        created_at (datetime.datetime):
        organization_name (Union[None, Unset, str]): Name of the Organization
        organization_details (Union['OrganizationResponseOrganizationDetailsType0', None, Unset]): Details about the
            organization
        metadata (Union['OrganizationResponseMetadataType0', None, Unset]): Metadata is useful for storing additional,
            unstructured information on an object.
    """

    organization_id: str
    modified_at: datetime.datetime
    created_at: datetime.datetime
    organization_name: Union[None, Unset, str] = UNSET
    organization_details: Union["OrganizationResponseOrganizationDetailsType0", None, Unset] = UNSET
    metadata: Union["OrganizationResponseMetadataType0", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.organization_response_metadata_type_0 import OrganizationResponseMetadataType0
        from ..models.organization_response_organization_details_type_0 import (
            OrganizationResponseOrganizationDetailsType0,
        )

        organization_id = self.organization_id

        modified_at = self.modified_at.isoformat()

        created_at = self.created_at.isoformat()

        organization_name: Union[None, Unset, str]
        if isinstance(self.organization_name, Unset):
            organization_name = UNSET
        else:
            organization_name = self.organization_name

        organization_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.organization_details, Unset):
            organization_details = UNSET
        elif isinstance(self.organization_details, OrganizationResponseOrganizationDetailsType0):
            organization_details = self.organization_details.to_dict()
        else:
            organization_details = self.organization_details

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, OrganizationResponseMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "organization_id": organization_id,
                "modified_at": modified_at,
                "created_at": created_at,
            }
        )
        if organization_name is not UNSET:
            field_dict["organization_name"] = organization_name
        if organization_details is not UNSET:
            field_dict["organization_details"] = organization_details
        if metadata is not UNSET:
            field_dict["metadata"] = metadata

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.organization_response_metadata_type_0 import OrganizationResponseMetadataType0
        from ..models.organization_response_organization_details_type_0 import (
            OrganizationResponseOrganizationDetailsType0,
        )

        d = dict(src_dict)
        organization_id = d.pop("organization_id")

        modified_at = isoparse(d.pop("modified_at"))

        created_at = isoparse(d.pop("created_at"))

        def _parse_organization_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        organization_name = _parse_organization_name(d.pop("organization_name", UNSET))

        def _parse_organization_details(
            data: object,
        ) -> Union["OrganizationResponseOrganizationDetailsType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                organization_details_type_0 = OrganizationResponseOrganizationDetailsType0.from_dict(data)

                return organization_details_type_0
            except:  # noqa: E722
                pass
            return cast(Union["OrganizationResponseOrganizationDetailsType0", None, Unset], data)

        organization_details = _parse_organization_details(d.pop("organization_details", UNSET))

        def _parse_metadata(data: object) -> Union["OrganizationResponseMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = OrganizationResponseMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["OrganizationResponseMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        organization_response = cls(
            organization_id=organization_id,
            modified_at=modified_at,
            created_at=created_at,
            organization_name=organization_name,
            organization_details=organization_details,
            metadata=metadata,
        )

        organization_response.additional_properties = d
        return organization_response

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
