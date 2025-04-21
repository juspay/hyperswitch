import datetime
from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.address_details import AddressDetails
    from ..models.customer_response_metadata_type_0 import CustomerResponseMetadataType0


T = TypeVar("T", bound="CustomerResponse")


@_attrs_define
class CustomerResponse:
    """
    Attributes:
        customer_id (str): The identifier for the customer object Example: cus_y3oqhf46pyzuxjbcn2giaqnb44.
        created_at (datetime.datetime): A timestamp (ISO 8601 code) that determines when the customer was created
            Example: 2023-01-18T11:04:09.922Z.
        name (Union[None, Unset, str]): The customer's name Example: Jon Test.
        email (Union[None, Unset, str]): The customer's email address Example: JonTest@test.com.
        phone (Union[None, Unset, str]): The customer's phone number Example: 9123456789.
        phone_country_code (Union[None, Unset, str]): The country code for the customer phone number Example: +65.
        description (Union[None, Unset, str]): An arbitrary string that you can attach to a customer object. Example:
            First Customer.
        address (Union['AddressDetails', None, Unset]):
        metadata (Union['CustomerResponseMetadataType0', None, Unset]): You can specify up to 50 keys, with key names up
            to 40 characters long and values up to 500
            characters long. Metadata is useful for storing additional, structured information on an
            object.
        default_payment_method_id (Union[None, Unset, str]): The identifier for the default payment method. Example:
            pm_djh2837dwduh890123.
    """

    customer_id: str
    created_at: datetime.datetime
    name: Union[None, Unset, str] = UNSET
    email: Union[None, Unset, str] = UNSET
    phone: Union[None, Unset, str] = UNSET
    phone_country_code: Union[None, Unset, str] = UNSET
    description: Union[None, Unset, str] = UNSET
    address: Union["AddressDetails", None, Unset] = UNSET
    metadata: Union["CustomerResponseMetadataType0", None, Unset] = UNSET
    default_payment_method_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.address_details import AddressDetails
        from ..models.customer_response_metadata_type_0 import CustomerResponseMetadataType0

        customer_id = self.customer_id

        created_at = self.created_at.isoformat()

        name: Union[None, Unset, str]
        if isinstance(self.name, Unset):
            name = UNSET
        else:
            name = self.name

        email: Union[None, Unset, str]
        if isinstance(self.email, Unset):
            email = UNSET
        else:
            email = self.email

        phone: Union[None, Unset, str]
        if isinstance(self.phone, Unset):
            phone = UNSET
        else:
            phone = self.phone

        phone_country_code: Union[None, Unset, str]
        if isinstance(self.phone_country_code, Unset):
            phone_country_code = UNSET
        else:
            phone_country_code = self.phone_country_code

        description: Union[None, Unset, str]
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        address: Union[None, Unset, dict[str, Any]]
        if isinstance(self.address, Unset):
            address = UNSET
        elif isinstance(self.address, AddressDetails):
            address = self.address.to_dict()
        else:
            address = self.address

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, CustomerResponseMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        default_payment_method_id: Union[None, Unset, str]
        if isinstance(self.default_payment_method_id, Unset):
            default_payment_method_id = UNSET
        else:
            default_payment_method_id = self.default_payment_method_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "customer_id": customer_id,
                "created_at": created_at,
            }
        )
        if name is not UNSET:
            field_dict["name"] = name
        if email is not UNSET:
            field_dict["email"] = email
        if phone is not UNSET:
            field_dict["phone"] = phone
        if phone_country_code is not UNSET:
            field_dict["phone_country_code"] = phone_country_code
        if description is not UNSET:
            field_dict["description"] = description
        if address is not UNSET:
            field_dict["address"] = address
        if metadata is not UNSET:
            field_dict["metadata"] = metadata
        if default_payment_method_id is not UNSET:
            field_dict["default_payment_method_id"] = default_payment_method_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.address_details import AddressDetails
        from ..models.customer_response_metadata_type_0 import CustomerResponseMetadataType0

        d = dict(src_dict)
        customer_id = d.pop("customer_id")

        created_at = isoparse(d.pop("created_at"))

        def _parse_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        name = _parse_name(d.pop("name", UNSET))

        def _parse_email(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        email = _parse_email(d.pop("email", UNSET))

        def _parse_phone(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        phone = _parse_phone(d.pop("phone", UNSET))

        def _parse_phone_country_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        phone_country_code = _parse_phone_country_code(d.pop("phone_country_code", UNSET))

        def _parse_description(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        description = _parse_description(d.pop("description", UNSET))

        def _parse_address(data: object) -> Union["AddressDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                address_type_1 = AddressDetails.from_dict(data)

                return address_type_1
            except:  # noqa: E722
                pass
            return cast(Union["AddressDetails", None, Unset], data)

        address = _parse_address(d.pop("address", UNSET))

        def _parse_metadata(data: object) -> Union["CustomerResponseMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = CustomerResponseMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["CustomerResponseMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        def _parse_default_payment_method_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        default_payment_method_id = _parse_default_payment_method_id(d.pop("default_payment_method_id", UNSET))

        customer_response = cls(
            customer_id=customer_id,
            created_at=created_at,
            name=name,
            email=email,
            phone=phone,
            phone_country_code=phone_country_code,
            description=description,
            address=address,
            metadata=metadata,
            default_payment_method_id=default_payment_method_id,
        )

        customer_response.additional_properties = d
        return customer_response

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
