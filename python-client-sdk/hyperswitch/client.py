from typing import Dict, Any, List, Optional, Union
from .models.merchant_connector_response import MerchantConnectorResponse
from .models.simple_merchant_connector_create import SimpleMerchantConnectorCreate

class Client:
    def create_merchant_connector_simple(
        self,
        merchant_id: str,
        connector_type: str,
        connector_name: str,
        connector_account_details: Dict[str, Any],
        connector_label: Optional[str] = None,
        profile_id: Optional[str] = None,
        payment_methods_enabled: Optional[List[Dict[str, Any]]] = None,
        test_mode: Optional[bool] = None,
        disabled: Optional[bool] = None,
        business_country: Optional[str] = None,
        business_label: Optional[str] = None,
        business_sub_label: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None
    ) -> MerchantConnectorResponse:
        """
        Create a new merchant connector using simplified direct serialization.
        
        Args:
            merchant_id: The ID of the merchant
            connector_type: Type of the connector (e.g. payment_processor)
            connector_name: Name of the connector (e.g. stripe)
            connector_account_details: Direct connector account details without nesting
            connector_label: Optional unique label to identify this connector account
            profile_id: Optional identifier for the profile
            payment_methods_enabled: Optional payment methods to enable
            test_mode: Optional flag to enable test mode
            disabled: Optional flag to disable the connector
            business_country: Optional country code for business
            business_label: Optional business label
            business_sub_label: Optional business sub-label
            metadata: Optional additional metadata
            
        Returns:
            MerchantConnectorResponse: The created merchant connector
        """
        request = SimpleMerchantConnectorCreate(
            connector_type=connector_type,
            connector_name=connector_name,
            connector_account_details=connector_account_details,
            connector_label=connector_label,
            profile_id=profile_id,
            payment_methods_enabled=payment_methods_enabled,
            test_mode=test_mode,
            disabled=disabled,
            business_country=business_country,
            business_label=business_label,
            business_sub_label=business_sub_label,
            metadata=metadata
        )
        
        response = self._post(
            f"/merchants/{merchant_id}/connectors",
            data=request.to_dict()
        )
        return MerchantConnectorResponse.from_json(response.text) 