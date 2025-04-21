from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.payments_update_metadata_response_metadata_type_0 import PaymentsUpdateMetadataResponseMetadataType0


T = TypeVar("T", bound="PaymentsUpdateMetadataResponse")


@_attrs_define
class PaymentsUpdateMetadataResponse:
    """
    Attributes:
        payment_id (str): The identifier for the payment
        metadata (Union['PaymentsUpdateMetadataResponseMetadataType0', None, Unset]): Metadata is useful for storing
            additional, unstructured information on an object.
    """

    payment_id: str
    metadata: Union["PaymentsUpdateMetadataResponseMetadataType0", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.payments_update_metadata_response_metadata_type_0 import (
            PaymentsUpdateMetadataResponseMetadataType0,
        )

        payment_id = self.payment_id

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, PaymentsUpdateMetadataResponseMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_id": payment_id,
            }
        )
        if metadata is not UNSET:
            field_dict["metadata"] = metadata

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.payments_update_metadata_response_metadata_type_0 import (
            PaymentsUpdateMetadataResponseMetadataType0,
        )

        d = dict(src_dict)
        payment_id = d.pop("payment_id")

        def _parse_metadata(data: object) -> Union["PaymentsUpdateMetadataResponseMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = PaymentsUpdateMetadataResponseMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["PaymentsUpdateMetadataResponseMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        payments_update_metadata_response = cls(
            payment_id=payment_id,
            metadata=metadata,
        )

        payments_update_metadata_response.additional_properties = d
        return payments_update_metadata_response

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
