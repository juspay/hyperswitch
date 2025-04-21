from typing import Dict, Any, List, Optional
from dataclasses import dataclass, field, asdict
from ..utils import to_camel_case, serialize_dict

@dataclass
class ConnectorAccountDetails:
    """Details for connecting to the payment processor."""
    auth_type: str
    api_key: str

@dataclass
class SimpleMerchantConnectorCreate:
    """
    A simplified model for creating a merchant connector that directly serializes connector details.
    """
    connector_type: str
    connector_name: str
    connector_account_details: Dict[str, str]
    connector_label: Optional[str] = None
    profile_id: Optional[str] = None
    payment_methods_enabled: Optional[List[Dict]] = None
    test_mode: Optional[bool] = None
    disabled: Optional[bool] = None
    business_country: Optional[str] = None
    business_label: Optional[str] = None
    business_sub_label: Optional[str] = None
    metadata: Optional[Dict] = None

    def to_dict(self) -> dict:
        """Convert the model instance to a dictionary, excluding None values."""
        data = asdict(self)
        # Remove None values
        return {k: v for k, v in data.items() if v is not None} 