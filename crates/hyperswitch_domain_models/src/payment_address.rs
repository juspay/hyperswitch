use api_models::payments::Address;

#[derive(Clone, Default, Debug)]
pub struct PaymentAddress {
    shipping: Option<Address>,
    billing: Option<Address>,
    unified_payment_method_billing: Option<Address>,
    payment_method_billing: Option<Address>,
}

impl PaymentAddress {
    pub fn new(
        shipping: Option<Address>,
        billing: Option<Address>,
        payment_method_billing: Option<Address>,
    ) -> Self {
        // billing -> .billing, this is the billing details passed in the root of payments request
        // payment_method_billing -> .payment_method_data.billing

        // Merge the billing details field from both `payment.billing` and `payment.payment_method_data.billing`
        // The unified payment_method_billing will be used as billing address and passed to the connector module
        // This unification is required in order to provide backwards compatibility
        // so that if `payment.billing` is passed it should be sent to the connector module
        // Unify the billing details with `payment_method_data.billing`
        let unified_payment_method_billing = payment_method_billing
            .as_ref()
            .map(|payment_method_billing| {
                payment_method_billing
                    .clone()
                    .unify_address(billing.as_ref())
            })
            .or(billing.clone());

        Self {
            shipping,
            billing,
            unified_payment_method_billing,
            payment_method_billing,
        }
    }

    pub fn get_shipping(&self) -> Option<&Address> {
        self.shipping.as_ref()
    }

    pub fn get_payment_method_billing(&self) -> Option<&Address> {
        self.unified_payment_method_billing.as_ref()
    }

    /// Unify the billing details from `payment_method_data.[payment_method_data].billing details`.
    pub fn unify_with_payment_method_data_billing(
        self,
        payment_method_data_billing: Option<Address>,
    ) -> Self {
        // Unify the billing details with `payment_method_data.billing_details`
        let unified_payment_method_billing = payment_method_data_billing
            .map(|payment_method_data_billing| {
                payment_method_data_billing.unify_address(self.get_payment_method_billing())
            })
            .or(self.get_payment_method_billing().cloned());

        Self {
            shipping: self.shipping,
            billing: self.billing,
            unified_payment_method_billing,
            payment_method_billing: self.payment_method_billing,
        }
    }

    pub fn get_request_payment_method_billing(&self) -> Option<&Address> {
        self.payment_method_billing.as_ref()
    }

    pub fn get_payment_billing(&self) -> Option<&Address> {
        self.billing.as_ref()
    }
}
