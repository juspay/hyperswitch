use std::fmt::Debug;

/// Represents an incoming payment API request from external clients
#[derive(Debug)]
pub struct PaymentApiRequest {
    payment_method_data: APIPaymentMethodData,
}

/// Enum representing different types of payment method data
#[derive(Debug)]
pub enum APIPaymentMethodData {
    Card(APICardData),
    CardVaultData(APICardVaultData),
}

/// Represents the processing flow type for payment operations
#[derive(Debug, Clone)]
pub enum Flow {
    Proxy,
    Normal,
}

/// API-level structure for raw card data
#[derive(Debug)]
pub struct APICardData {
    card_number: i64,
    card_expiry: i64,
    card_cvc: i64,
}

/// API-level structure for vault token data
#[derive(Debug)]
pub struct APICardVaultData {
    card_number: String,
    card_expiry: String,
    card_cvc: String,
}

/// Domain-level enum for payment method data with generic PII handling
#[derive(Debug)]
pub enum DomainPaymentMethodData<T: PIIHolder> {
    Card(Card<T>),
    CardVaultData(Card<T>),
}

/// Trait defining the types used for storing different kinds of PII
pub trait PIIHolder {
    type CardNum: Default + Debug;
    type CardCvc: Default + Debug;
    type CardExpiry: Default + Debug;
}

/// Trait for defining inner data types used in PII processing
pub trait PIIInner {
    type Inner: Default + Debug;
}

/// Generic card structure that can hold different types of card data
#[derive(Default, Debug)]
pub struct Card<T: PIIHolder> {
    card_number: T::CardNum,
    card_expiry: T::CardExpiry,
    card_cvc: T::CardCvc,
}

/// PII holder implementation for handling raw PCI data
#[derive(Default, Debug)]
pub struct DefaultPCIHolder;

impl PIIHolder for DefaultPCIHolder {
    type CardNum = i64;
    type CardExpiry = i64;
    type CardCvc = i64;
}

impl PIIInner for DefaultPCIHolder {
    type Inner = String;
}

/// PII holder implementation for handling vault token data
#[derive(Default, Debug)]
pub struct VaultTokenHolder;

impl PIIHolder for VaultTokenHolder {
    type CardNum = String;
    type CardExpiry = String;
    type CardCvc = String;
}

impl PIIInner for VaultTokenHolder {
    type Inner = u8;
}

/// Router data structure containing processed payment information
#[derive(Debug)]
pub struct RouterData<T: PIIHolder> {
    pub payment_method_data: DomainPaymentMethodData<T>,
    pub flow: Flow,
}

/// Tracker structure for monitoring payment processing
#[derive(Debug)]
pub struct Tracker<T: PIIHolder> {
    pub payment_method_data: DomainPaymentMethodData<T>,
    pub flow: Flow,
}

/// Error type for conversion failures
#[derive(Debug)]
pub struct ConversionError {
    message: String,
}

impl ConversionError {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Conversion error: {}", self.message)
    }
}

impl std::error::Error for ConversionError {}

// === FROM TRAIT IMPLEMENTATIONS ===

impl From<APICardData> for Card<DefaultPCIHolder> {
    fn from(api_card: APICardData) -> Self {
        Card {
            card_number: api_card.card_number,
            card_expiry: api_card.card_expiry,
            card_cvc: api_card.card_cvc,
        }
    }
}

impl From<APICardVaultData> for Card<VaultTokenHolder> {
    fn from(api_vault: APICardVaultData) -> Self {
        Card {
            card_number: api_vault.card_number,
            card_expiry: api_vault.card_expiry,
            card_cvc: api_vault.card_cvc,
        }
    }
}

// === TRY FROM IMPLEMENTATIONS FOR INCOMPATIBLE CONVERSIONS ===

impl TryFrom<APICardData> for Card<VaultTokenHolder> {
    type Error = ConversionError;

    fn try_from(_api_card: APICardData) -> Result<Self, Self::Error> {
        Err(ConversionError::new("Cannot convert raw card data to vault token holder"))
    }
}

impl TryFrom<APICardVaultData> for Card<DefaultPCIHolder> {
    type Error = ConversionError;

    fn try_from(_api_vault: APICardVaultData) -> Result<Self, Self::Error> {
        Err(ConversionError::new("Cannot convert vault token data to raw PCI holder"))
    }
}

// === CORE PAYMENT PROCESSING FUNCTIONS ===

pub fn payments_operation_core<T: PIIHolder + Debug>(tracker_data: Tracker<T>) {
    let router_data = RouterData {
        payment_method_data: tracker_data.payment_method_data,
        flow: tracker_data.flow,
    };
    
    println!("Router data: {:?}", router_data);
}

// ===================================================================
// APPROACH 1: GENERIC FUNCTION
// ===================================================================

/// **APPROACH 1: Generic Function where caller specifies the type**
/// 
/// This approach requires the caller to specify the PIIHolder type at compile time.
/// The function will only succeed if the API request data matches the specified type.
/// 
/// # Type Parameters
/// * `T` - The PIIHolder type that determines how PII data is stored
/// 
/// # Arguments
/// * `req` - The incoming payment API request
/// 
/// # Returns
/// * `Option<Tracker<T>>` - Some(tracker) if the request matches type T, None otherwise
/// 
/// # Usage Examples
/// ```rust
/// // Caller must specify the type they expect
/// let tracker: Option<Tracker<DefaultPCIHolder>> = get_trackers_generic(req);
/// let tracker: Option<Tracker<VaultTokenHolder>> = get_trackers_generic(req);
/// ```
/// 
/// # Limitations
/// - Caller must know the expected type at compile time
/// - Returns None if the API request doesn't match the expected type
/// - Requires separate calls for different types
pub fn get_trackers_generic<T: PIIHolder + Debug>(req: PaymentApiRequest) -> Option<Tracker<T>>
where
    Card<T>: TryFrom<APICardData> + TryFrom<APICardVaultData>,
{
    match req.payment_method_data {
        APIPaymentMethodData::Card(api_card_data) => {
            // Try to convert API card data to the specified type T
            if let Ok(card) = Card::<T>::try_from(api_card_data) {
                let tracker = Tracker {
                    payment_method_data: DomainPaymentMethodData::Card(card),
                    flow: Flow::Normal,
                };
                println!("Generic: Created card tracker for type: {:?}", std::any::type_name::<T>());
                Some(tracker)
            } else {
                println!("Generic: Failed to convert APICardData to type: {}", std::any::type_name::<T>());
                None
            }
        }
        APIPaymentMethodData::CardVaultData(api_vault_data) => {
            // Try to convert API vault data to the specified type T
            if let Ok(card) = Card::<T>::try_from(api_vault_data) {
                let tracker = Tracker {
                    payment_method_data: DomainPaymentMethodData::CardVaultData(card),
                    flow: Flow::Proxy,
                };
                println!("Generic: Created vault tracker for type: {:?}", std::any::type_name::<T>());
                Some(tracker)
            } else {
                println!("Generic: Failed to convert APICardVaultData to type: {}", std::any::type_name::<T>());
                None
            }
        }
    }
}

// ===================================================================
// APPROACH 2: BOX<DYN TRAIT>
// ===================================================================

/// Trait for processing tracker data in a type-erased manner
/// This allows us to work with different Tracker types through a common interface
pub trait TrackerProcessor: Debug {
    /// Process the tracker data and create router data
    fn process(&self);
    
    /// Get the flow type for this tracker
    fn get_flow(&self) -> &Flow;
    
    /// Get a string representation of the PIIHolder type
    fn get_pii_holder_type(&self) -> &'static str;
}

/// Implementation of TrackerProcessor for any Tracker<T> where T: PIIHolder + Debug
impl<T: PIIHolder + Debug> TrackerProcessor for Tracker<T> {
    fn process(&self) {
        println!("BoxDyn: Processing tracker: {:?}", self);
        println!("BoxDyn: Flow type: {:?}", self.flow);
        println!("BoxDyn: PIIHolder type: {}", std::any::type_name::<T>());
    }
    
    fn get_flow(&self) -> &Flow {
        &self.flow
    }
    
    fn get_pii_holder_type(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

/// **APPROACH 2: Box<dyn Trait> for runtime type erasure**
/// 
/// This approach provides a unified interface that can handle any tracker type
/// at runtime without requiring the caller to specify the type beforehand.
/// 
/// # Arguments
/// * `req` - The incoming payment API request
/// 
/// # Returns
/// * `Box<dyn TrackerProcessor>` - A trait object that can process any tracker type
/// 
/// # Advantages
/// - Unified interface - single function handles all cases
/// - Runtime type determination based on API request content
/// - Type safety maintained through trait bounds
/// - Caller doesn't need to know specific types
/// 
/// # Usage
/// ```rust
/// let processor = get_trackers_boxed(req);
/// processor.process(); // Works regardless of underlying type
/// ```
pub fn get_trackers_boxed(req: PaymentApiRequest) -> Box<dyn TrackerProcessor> {
    match req.payment_method_data {
        APIPaymentMethodData::Card(api_card_data) => {
            // Convert API card data to domain card data using From trait
            let card: Card<DefaultPCIHolder> = api_card_data.into();
            
            // Create tracker for raw PCI data with Normal flow
            let tracker = Tracker {
                payment_method_data: DomainPaymentMethodData::Card(card),
                flow: Flow::Normal,
            };
            
            println!("BoxDyn: Created card tracker: {:?}", tracker);
            Box::new(tracker)
        }
        APIPaymentMethodData::CardVaultData(api_vault_data) => {
            // Convert API vault data to domain card data using From trait
            let card: Card<VaultTokenHolder> = api_vault_data.into();
            
            // Create tracker for vault token data with Proxy flow
            let tracker = Tracker {
                payment_method_data: DomainPaymentMethodData::CardVaultData(card),
                flow: Flow::Proxy,
            };
            
            println!("BoxDyn: Created vault tracker: {:?}", tracker);
            Box::new(tracker)
        }
    }
}

// === HELPER FUNCTIONS FOR DEMONSTRATION ===

/// Helper function to demonstrate generic approach usage
pub fn demonstrate_generic_approach(req: PaymentApiRequest) {
    println!("\n=== APPROACH 1: Generic Function Demo ===");
    
    // Try with DefaultPCIHolder
    println!("\n--- Trying with DefaultPCIHolder ---");
    if let Some(tracker) = get_trackers_generic::<DefaultPCIHolder>(req.clone()) {
        println!("✅ Success: Got Tracker<DefaultPCIHolder>");
        payments_operation_core(tracker);
    } else {
        println!("❌ Failed: API request doesn't match DefaultPCIHolder");
    }
    
    // Try with VaultTokenHolder
    println!("\n--- Trying with VaultTokenHolder ---");
    if let Some(tracker) = get_trackers_generic::<VaultTokenHolder>(req) {
        println!("✅ Success: Got Tracker<VaultTokenHolder>");
        payments_operation_core(tracker);
    } else {
        println!("❌ Failed: API request doesn't match VaultTokenHolder");
    }
}

/// Helper function to demonstrate Box<dyn> approach usage
pub fn demonstrate_boxed_approach(req: PaymentApiRequest) {
    println!("\n=== APPROACH 2: Box<dyn Trait> Demo ===");
    
    let processor = get_trackers_boxed(req);
    println!("✅ Success: Got Box<dyn TrackerProcessor>");
    println!("PIIHolder type: {}", processor.get_pii_holder_type());
    println!("Flow: {:?}", processor.get_flow());
    processor.process();
}

// We need to implement Clone for PaymentApiRequest to use it multiple times
impl Clone for PaymentApiRequest {
    fn clone(&self) -> Self {
        match &self.payment_method_data {
            APIPaymentMethodData::Card(card_data) => PaymentApiRequest {
                payment_method_data: APIPaymentMethodData::Card(APICardData {
                    card_number: card_data.card_number,
                    card_expiry: card_data.card_expiry,
                    card_cvc: card_data.card_cvc,
                }),
            },
            APIPaymentMethodData::CardVaultData(vault_data) => PaymentApiRequest {
                payment_method_data: APIPaymentMethodData::CardVaultData(APICardVaultData {
                    card_number: vault_data.card_number.clone(),
                    card_expiry: vault_data.card_expiry.clone(),
                    card_cvc: vault_data.card_cvc.clone(),
                }),
            },
        }
    }
}

// === MAIN FUNCTION FOR TESTING ===

fn main() {
    println!("=== Payment Processing System - Both Approaches ===\n");
    
    // === TEST CASE 1: Raw Card Data ===
    println!("🔹 TEST CASE 1: Raw Card Data");
    let api_req_card = PaymentApiRequest {
        payment_method_data: APIPaymentMethodData::Card(APICardData {
            card_number: 1234567890123456,
            card_expiry: 1225,
            card_cvc: 123,
        }),
    };
    
    demonstrate_generic_approach(api_req_card.clone());
    demonstrate_boxed_approach(api_req_card);
    
    println!("\n" + "=".repeat(80).as_str());
    
    // === TEST CASE 2: Vault Token Data ===
    println!("\n🔹 TEST CASE 2: Vault Token Data");
    let api_req_vault = PaymentApiRequest {
        payment_method_data: APIPaymentMethodData::CardVaultData(APICardVaultData {
            card_number: "token_1234567890123456".to_string(),
            card_expiry: "1225".to_string(),
            card_cvc: "123".to_string(),
        }),
    };
    
    demonstrate_generic_approach(api_req_vault.clone());
    demonstrate_boxed_approach(api_req_vault);
    
    // === COMPARISON SUMMARY ===
    println!("\n" + "=".repeat(80).as_str());
    println!("\n📊 **COMPARISON SUMMARY**");
    
    println!("\n🔸 **APPROACH 1: Generic Function**");
    println!("   ✅ Returns concrete Tracker<T> types");
    println!("   ✅ Zero runtime overhead");
    println!("   ✅ Compile-time type safety");
    println!("   ❌ Caller must specify type at compile time");
    println!("   ❌ Returns Option - can fail if types don't match");
    println!("   ❌ Requires multiple calls for different types");
    println!("   📝 Usage: let tracker = get_trackers_generic::<DefaultPCIHolder>(req);");
    
    println!("\n🔸 **APPROACH 2: Box<dyn Trait>**");
    println!("   ✅ Unified interface - single function call");
    println!("   ✅ Runtime type determination");
    println!("   ✅ Always succeeds (no Option)");
    println!("   ✅ Caller doesn't need to know types");
    println!("   ❌ Small runtime overhead (heap allocation + dynamic dispatch)");
    println!("   ❌ Type erasure - lose concrete type information");
    println!("   📝 Usage: let processor = get_trackers_boxed(req);");
    
    println!("\n🎯 **RECOMMENDATION**:");
    println!("   - Use **Generic Function** when you know the expected type at compile time");
    println!("   - Use **Box<dyn Trait>** for unified APIs and runtime type determination");
    println!("   - For most payment processing scenarios, **Box<dyn Trait>** is more practical");
}
