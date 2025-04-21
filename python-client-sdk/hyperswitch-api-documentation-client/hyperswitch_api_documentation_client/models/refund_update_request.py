from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.refund_update_request_metadata_type_0 import RefundUpdateRequestMetadataType0


T = TypeVar("T", bound="RefundUpdateRequest")


@_attrs_define
class RefundUpdateRequest:
    """
    Attributes:
        reason (Union[None, Unset, str]): An arbitrary string attached to the object. Often useful for displaying to
            users and your customer support executive Example: Customer returned the product.
        metadata (Union['RefundUpdateRequestMetadataType0', None, Unset]): You can specify up to 50 keys, with key names
            up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional,
            structured information on an object.
    """

    reason: Union[None, Unset, str] = UNSET
    metadata: Union["RefundUpdateRequestMetadataType0", None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.refund_update_request_metadata_type_0 import RefundUpdateRequestMetadataType0

        reason: Union[None, Unset, str]
        if isinstance(self.reason, Unset):
            reason = UNSET
        else:
            reason = self.reason

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, RefundUpdateRequestMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        field_dict: dict[str, Any] = {}
        field_dict.update({})
        if reason is not UNSET:
            field_dict["reason"] = reason
        if metadata is not UNSET:
            field_dict["metadata"] = metadata

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.refund_update_request_metadata_type_0 import RefundUpdateRequestMetadataType0

        d = dict(src_dict)

        def _parse_reason(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        reason = _parse_reason(d.pop("reason", UNSET))

        def _parse_metadata(data: object) -> Union["RefundUpdateRequestMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = RefundUpdateRequestMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["RefundUpdateRequestMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        refund_update_request = cls(
            reason=reason,
            metadata=metadata,
        )

        return refund_update_request
