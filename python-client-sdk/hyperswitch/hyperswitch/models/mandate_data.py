from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.customer_acceptance import CustomerAcceptance
    from ..models.mandate_type_type_0 import MandateTypeType0
    from ..models.mandate_type_type_1 import MandateTypeType1


T = TypeVar("T", bound="MandateData")


@_attrs_define
class MandateData:
    """Passing this object during payments creates a mandate. The mandate_type sub object is passed by the server.

    Attributes:
        update_mandate_id (Union[None, Unset, str]): A way to update the mandate's payment method details
        customer_acceptance (Union['CustomerAcceptance', None, Unset]):
        mandate_type (Union['MandateTypeType0', 'MandateTypeType1', None, Unset]):
    """

    update_mandate_id: Union[None, Unset, str] = UNSET
    customer_acceptance: Union["CustomerAcceptance", None, Unset] = UNSET
    mandate_type: Union["MandateTypeType0", "MandateTypeType1", None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.customer_acceptance import CustomerAcceptance
        from ..models.mandate_type_type_0 import MandateTypeType0
        from ..models.mandate_type_type_1 import MandateTypeType1

        update_mandate_id: Union[None, Unset, str]
        if isinstance(self.update_mandate_id, Unset):
            update_mandate_id = UNSET
        else:
            update_mandate_id = self.update_mandate_id

        customer_acceptance: Union[None, Unset, dict[str, Any]]
        if isinstance(self.customer_acceptance, Unset):
            customer_acceptance = UNSET
        elif isinstance(self.customer_acceptance, CustomerAcceptance):
            customer_acceptance = self.customer_acceptance.to_dict()
        else:
            customer_acceptance = self.customer_acceptance

        mandate_type: Union[None, Unset, dict[str, Any]]
        if isinstance(self.mandate_type, Unset):
            mandate_type = UNSET
        elif isinstance(self.mandate_type, MandateTypeType0):
            mandate_type = self.mandate_type.to_dict()
        elif isinstance(self.mandate_type, MandateTypeType1):
            mandate_type = self.mandate_type.to_dict()
        else:
            mandate_type = self.mandate_type

        field_dict: dict[str, Any] = {}
        field_dict.update({})
        if update_mandate_id is not UNSET:
            field_dict["update_mandate_id"] = update_mandate_id
        if customer_acceptance is not UNSET:
            field_dict["customer_acceptance"] = customer_acceptance
        if mandate_type is not UNSET:
            field_dict["mandate_type"] = mandate_type

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.customer_acceptance import CustomerAcceptance
        from ..models.mandate_type_type_0 import MandateTypeType0
        from ..models.mandate_type_type_1 import MandateTypeType1

        d = dict(src_dict)

        def _parse_update_mandate_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        update_mandate_id = _parse_update_mandate_id(d.pop("update_mandate_id", UNSET))

        def _parse_customer_acceptance(data: object) -> Union["CustomerAcceptance", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                customer_acceptance_type_1 = CustomerAcceptance.from_dict(data)

                return customer_acceptance_type_1
            except:  # noqa: E722
                pass
            return cast(Union["CustomerAcceptance", None, Unset], data)

        customer_acceptance = _parse_customer_acceptance(d.pop("customer_acceptance", UNSET))

        def _parse_mandate_type(data: object) -> Union["MandateTypeType0", "MandateTypeType1", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_mandate_type_type_0 = MandateTypeType0.from_dict(data)

                return componentsschemas_mandate_type_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_mandate_type_type_1 = MandateTypeType1.from_dict(data)

                return componentsschemas_mandate_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MandateTypeType0", "MandateTypeType1", None, Unset], data)

        mandate_type = _parse_mandate_type(d.pop("mandate_type", UNSET))

        mandate_data = cls(
            update_mandate_id=update_mandate_id,
            customer_acceptance=customer_acceptance,
            mandate_type=mandate_type,
        )

        return mandate_data
