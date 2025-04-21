from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.organization_create_request_metadata_type_0 import OrganizationCreateRequestMetadataType0
    from ..models.organization_create_request_organization_details_type_0 import (
        OrganizationCreateRequestOrganizationDetailsType0,
    )


T = TypeVar("T", bound="OrganizationCreateRequest")


@_attrs_define
class OrganizationCreateRequest:
    """
    Attributes:
        organization_name (str): Name of the organization
        organization_details (Union['OrganizationCreateRequestOrganizationDetailsType0', None, Unset]): Details about
            the organization
        metadata (Union['OrganizationCreateRequestMetadataType0', None, Unset]): Metadata is useful for storing
            additional, unstructured information on an object.
    """

    organization_name: str
    organization_details: Union["OrganizationCreateRequestOrganizationDetailsType0", None, Unset] = UNSET
    metadata: Union["OrganizationCreateRequestMetadataType0", None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.organization_create_request_metadata_type_0 import OrganizationCreateRequestMetadataType0
        from ..models.organization_create_request_organization_details_type_0 import (
            OrganizationCreateRequestOrganizationDetailsType0,
        )

        organization_name = self.organization_name

        organization_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.organization_details, Unset):
            organization_details = UNSET
        elif isinstance(self.organization_details, OrganizationCreateRequestOrganizationDetailsType0):
            organization_details = self.organization_details.to_dict()
        else:
            organization_details = self.organization_details

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, OrganizationCreateRequestMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "organization_name": organization_name,
            }
        )
        if organization_details is not UNSET:
            field_dict["organization_details"] = organization_details
        if metadata is not UNSET:
            field_dict["metadata"] = metadata

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.organization_create_request_metadata_type_0 import OrganizationCreateRequestMetadataType0
        from ..models.organization_create_request_organization_details_type_0 import (
            OrganizationCreateRequestOrganizationDetailsType0,
        )

        d = dict(src_dict)
        organization_name = d.pop("organization_name")

        def _parse_organization_details(
            data: object,
        ) -> Union["OrganizationCreateRequestOrganizationDetailsType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                organization_details_type_0 = OrganizationCreateRequestOrganizationDetailsType0.from_dict(data)

                return organization_details_type_0
            except:  # noqa: E722
                pass
            return cast(Union["OrganizationCreateRequestOrganizationDetailsType0", None, Unset], data)

        organization_details = _parse_organization_details(d.pop("organization_details", UNSET))

        def _parse_metadata(data: object) -> Union["OrganizationCreateRequestMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = OrganizationCreateRequestMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["OrganizationCreateRequestMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        organization_create_request = cls(
            organization_name=organization_name,
            organization_details=organization_details,
            metadata=metadata,
        )

        return organization_create_request
