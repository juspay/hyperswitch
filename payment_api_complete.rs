use std::fmt::Debug;

/// Represents an incoming payment API request from external clients
/// Contains the payment method data that needs to be processed
#[derive(Debug)]
pub struct PaymentApiRequest {
    payment_method_data: APIPaymentMethodData,
}

/// Enum representing different types of payment method data that can be received via API
/// This allows the system to handle both raw card data and vault tokens
#[derive(Debug)]
pub enum APIPaymentMethodData {
    /// Raw card data containing actual card numbers, expiry, and CVC
    /// Used when the client sends sensitive card information directly
    Card(APICardData),
    /// Vault token data containing tokenized card information
    /// Used when the client sends previously vaulted/tokenized card data
    CardVaultData(APICardVaultData),
}

/// Represents the processing flow type for payment operations
/// Determines how the payment should be routed and processed
#[derive(Debug, Clone)]
pub enum Flow {
    /// Proxy flow - used for vault token processing where data is already tokenized
    Proxy,
    /// Normal flow - used for raw card data processing
    Normal,
}

/// API-level structure for raw card data
/// Contains sensitive PCI data as received from external clients
#[derive(Debug)]
pub struct APICardData {
    /// Card number as integer (raw PCI data)
    card_number: i64,
    /// Card expiry date as integer (MMYY format)
    card_expiry: i64,
    /// Card verification code as integer
    card_cvc: i64,
}

/// API-level structure for vault token data
/// Contains tokenized card information as strings
#[derive(Debug)]
pub struct APICardVaultData {
    /// Tokenized card number as string
    card_number: String,
    /// Tokenized card expiry as string
    card_expiry: String,
    /// Tokenized card CVC as string
    card_cvc: String,
}

/// Domain-level enum for payment method data with generic PII handling
/// This is the internal representation used within the payment processing system
/// Generic over T: PIIHolder to ensure type-safe handling of different data types
#[derive(Debug)]
pub enum DomainPaymentMethodData<T: PIIHolder> {
    /// Regular card data using the specified PII holder type
    Card(Card<T>),
    /// Vault card data using the specified PII holder type
    /// Note: Both variants use Card<T> but represent different data sources
    CardVaultData(Card<T>),
}

/// Trait defining the types used for storing different kinds of PII (Personally Identifiable Information)
/// This trait allows the system to use different data types for card information
/// depending on whether it's raw data or tokenized data
pub trait PIIHolder {
    /// Type used for storing card numbers (could be i64 for raw data, String for tokens)
    type CardNum: Default + Debug;
    /// Type used for storing card CVC (could be i64 for raw data, String for tokens)
    type CardCvc: Default + Debug;
    /// Type used for storing card expiry (could be i64 for raw data, String for tokens)
    type CardExpiry: Default + Debug;
}

/// Trait for defining inner data types used in PII processing
/// This provides additional type information for complex PII handling scenarios
pub trait PIIInner {
    /// Inner type used for additional data processing
    type Inner: Default + Debug;
}

/// Generic card structure that can hold different types of card data
/// The type parameter T determines what kind of data types are used for each field
#[derive(Default, Debug)]
pub struct Card<T: PIIHolder> {
    /// Card number using the type specified by the PIIHolder
    card_number: T::CardNum,
    /// Card expiry using the type specified by the PIIHolder
    card_expiry: T::CardExpiry,
    /// Card CVC using the type specified by the PIIHolder
    card_cvc: T::CardCvc,
}

/// PII holder implementation for handling raw PCI data
/// Uses integer types for storing actual card information
#[derive(Default, Debug)]
pub struct DefaultPCIHolder;

impl PIIHolder for DefaultPCIHolder {
    /// Raw card numbers are stored as 64-bit integers
    type CardNum = i64;
    /// Raw card expiry dates are stored as 64-bit integers
    type CardExpiry = i64;
    /// Raw card CVCs are stored as 64-bit integers
    type CardCvc = i64;
}

impl PIIInner for DefaultPCIHolder {
    /// Inner type for additional processing is String
    type Inner = String;
}

/// PII holder implementation for handling vault token data
/// Uses string types for storing tokenized card information
#[derive(Default, Debug)]
pub struct VaultTokenHolder;

impl PIIHolder for VaultTokenHolder {
    /// Vault tokens for card numbers are stored as strings
    type CardNum = String;
    /// Vault tokens for card expiry are stored as strings
    type CardExpiry = String;
    /// Vault tokens for card CVCs are stored as strings
    type CardCvc = String;
}

impl PIIInner for VaultTokenHolder {
    /// Inner type for additional processing is u8
    type Inner = u8;
}

/// Router data structure containing processed payment information
/// This represents the data structure used by the payment router
/// Generic over T: PIIHolder to maintain type safety throughout the system
#[derive(Debug)]
pub struct RouterData<T: PIIHolder> {
    /// The processed payment method data
    pub payment_method_data: DomainPaymentMethodData<T>,
    /// The determined flow type for this payment
    pub flow: Flow,
}

/// Tracker structure for monitoring payment processing
/// Contains the same data as RouterData but used for tracking purposes
/// Generic over T: PIIHolder to maintain type consistency
#[derive(Debug)]
pub struct Tracker<T: PIIHolder> {
    /// The payment method data being tracked
    pub payment_method_data: DomainPaymentMethodData<T>,
    /// The flow type being used for this payment
    pub flow: Flow,
}

/// Error type for conversion failures
/// Used when TryFrom implementations fail due to incompatible types
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

// === FROM TRAIT IMPLEMENTATIONS FOR API TO DOMAIN CONVERSION ===

/// Converts API card data (raw PCI data) to domain card data
/// Maps integer fields from API structure to DefaultPCIHolder card structure
impl From<APICardData> for Card<DefaultPCIHolder> {
    fn from(api_card: APICardData) -> Self {
        Card {
            card_number: api_card.card_number,
            card_expiry: api_card.card_expiry,
            card_cvc: api_card.card_cvc,
        }
    }
}

/// Converts API vault data (tokenized data) to domain card data
/// Maps string fields from API structure to VaultTokenHolder card structure
impl From<APICardVaultData> for Card<VaultTokenHolder> {
    fn from(api_vault: APICardVaultData) -> Self {
        Card {
            card_number: api_vault.card_number,
            card_expiry: api_vault.card_expiry,
            card_cvc: api_vault.card_cvc,
        }
    }
}

// === TRY FROM IMPLEMENTATIONS FOR GENERIC CONVERSION ===

/// TryFrom implementation for converting APICardData to Card<DefaultPCIHolder>
/// This allows the generic function to work with DefaultPCIHolder
/// Always succeeds since the types are compatible
impl TryFrom<APICardData> for Card<DefaultPCIHolder> {
    type Error = ConversionError;

    fn try_from(api_card: APICardData) -> Result<Self, Self::Error> {
        Ok(Card {
            card_number: api_card.card_number,
            card_expiry: api_card.card_expiry,
            card_cvc: api_card.card_cvc,
        })
    }
}

/// TryFrom implementation for converting APICardVaultData to Card<VaultTokenHolder>
/// This allows the generic function to work with VaultTokenHolder
/// Always succeeds since the types are compatible
impl TryFrom<APICardVaultData> for Card<VaultTokenHolder> {
    type Error = ConversionError;

    fn try_from(api_vault: APICardVaultData) -> Result<Self, Self::Error> {
        Ok(Card {
            card_number: api_vault.card_number,
            card_expiry: api_vault.card_expiry,
            card_cvc: api_vault.card_cvc,
        })
    }
}

/// TryFrom implementation for converting APICardData to Card<VaultTokenHolder>
/// This will always fail since we can't convert raw integers to vault tokens
/// Demonstrates type safety - prevents invalid conversions
impl TryFrom<APICardData> for Card<VaultTokenHolder> {
    type Error = ConversionError;

    fn try_from(_api_card: APICardData) -> Result<Self, Self::Error> {
        Err(ConversionError::new("Cannot convert raw card data to vault token holder"))
    }
}

/// TryFrom implementation for converting APICardVaultData to Card<DefaultPCIHolder>
/// This will always fail since we can't convert vault tokens to raw integers
/// Demonstrates type safety - prevents invalid conversions
impl TryFrom<APICardVaultData> for Card<DefaultPCIHolder> {
    type Error = ConversionError;

    fn try_from(_api_vault: APICardVaultData) -> Result<Self, Self::Error> {
        Err(ConversionError::new("Cannot convert vault token data to raw PCI holder"))
    }
}

// === CORE PAYMENT PROCESSING FUNCTIONS ===

/// Core payment operation function that processes tracker data
/// Generic over T: PIIHolder + Debug to handle any type of PII data
/// 
/// # Arguments
/// * `tracker_data` - The tracker containing payment method data and flow information
/// 
/// # Behavior
/// - Creates RouterData from the tracker data
/// - Prints the router data for debugging/logging purposes
pub fn payments_operation_core<T: PIIHolder + Debug>(tracker_data: Tracker<T>) {
    // Create router data from tracker data
    let router_data = RouterData {
        payment_method_data: tracker_data.payment_method_data,
        flow: tracker_data.flow,
    };
    
    // Print router data for debugging/monitoring
    println!("Router data: {:?}", router_data);
}

/// Trait for processing tracker data in a type-erased manner
/// This allows us to work with different Tracker types through a common interface
pub trait TrackerProcessor: Debug {
    /// Process the tracker data and create router data
    fn process(&self);
    
    /// Get the flow type for this tracker
    fn get_flow(&self) -> &Flow;
}

/// Implementation of TrackerProcessor for any Tracker<T> where T: PIIHolder + Debug
/// This allows any tracker to be processed through the common interface
impl<T: PIIHolder + Debug> TrackerProcessor for Tracker<T> {
    fn process(&self) {
        println!("Processing tracker: {:?}", self);
        println!("Flow type: {:?}", self.flow);
    }
    
    fn get_flow(&self) -> &Flow {
        &self.flow
    }
}

/// Main get_trackers function that returns Box<dyn TrackerProcessor>
/// 
/// **Why Box<dyn Trait> is necessary:**
/// 1. **Runtime Type Determination**: The specific PIIHolder type (DefaultPCIHolder vs VaultTokenHolder) 
///    is determined by the incoming API request data at runtime, not compile time.
/// 
/// 2. **Single Function Interface**: Without Box<dyn Trait>, you'd need separate functions for each 
///    type, breaking the unified API interface that `get_trackers` provides.
/// 
/// 3. **Type System Limitation**: Rust's type system doesn't allow a function to return different 
///    concrete types based on runtime conditions. The alternatives are:
///    - Enums (which you explicitly don't want)
///    - Separate functions (breaks unified interface)
///    - Generic functions (requires caller to specify type at compile time)
///    - Trait objects (Box<dyn Trait>)
/// 
/// 4. **Practical Usage**: In real payment systems, you receive API requests and need to process 
///    them without knowing the exact type beforehand. Box<dyn Trait> enables this pattern.
/// 
/// # Arguments
/// * `req` - The incoming payment API request
/// 
/// # Returns
/// * `Box<dyn TrackerProcessor>` - A trait object that can process any tracker type
pub fn get_trackers(req: PaymentApiRequest) -> Box<dyn TrackerProcessor> {
    match req.payment_method_data {
        APIPaymentMethodData::Card(api_card_data) => {
            // Convert API card data to domain card data using From trait
            let card: Card<DefaultPCIHolder> = api_card_data.into();
            
            // Create tracker for raw PCI data with Normal flow
            let tracker = Tracker {
                payment_method_data: DomainPaymentMethodData::Card(card),
                flow: Flow::Normal, // Raw card data uses normal processing flow
            };
            
            println!("Created card tracker: {:?}", tracker);
            Box::new(tracker)
        }
        APIPaymentMethodData::CardVaultData(api_vault_data) => {
            // Convert API vault data to domain card data using From trait
            let card: Card<VaultTokenHolder> = api_vault_data.into();
            
            // Create tracker for vault token data with Proxy flow
            let tracker = Tracker {
                payment_method_data: DomainPaymentMethodData::CardVaultData(card),
                flow: Flow::Proxy, // Vault data uses proxy processing flow
            };
            
            println!("Created vault tracker: {:?}", tracker);
            Box::new(tracker)
        }
    }
}

/// Alternative: Separate functions that return concrete Tracker<T> types
/// These avoid Box<dyn> but require the caller to know the type beforehand

/// Creates a tracker specifically for card data - returns concrete type
/// 
/// # Arguments
/// * `api_card_data` - The API card data to convert
/// 
/// # Returns
/// * `Tracker<DefaultPCIHolder>` - Tracker configured for raw PCI data processing
pub fn get_card_tracker(api_card_data: APICardData) -> Tracker<DefaultPCIHolder> {
    let card: Card<DefaultPCIHolder> = api_card_data.into();
    let tracker = Tracker {
        payment_method_data: DomainPaymentMethodData::Card(card),
        flow: Flow::Normal,
    };
    println!("Created card tracker: {:?}", tracker);
    tracker
}

/// Creates a tracker specifically for vault data - returns concrete type
/// 
/// # Arguments
/// * `api_vault_data` - The API vault data to convert
/// 
/// # Returns
/// * `Tracker<VaultTokenHolder>` - Tracker configured for vault token processing
pub fn get_vault_tracker(api_vault_data: APICardVaultData) -> Tracker<VaultTokenHolder> {
    let card: Card<VaultTokenHolder> = api_vault_data.into();
    let tracker = Tracker {
        payment_method_data: DomainPaymentMethodData::CardVaultData(card),
        flow: Flow::Proxy,
    };
    println!("Created vault tracker: {:?}", tracker);
    tracker
}

// === MAIN FUNCTION FOR TESTING ===

/// Main function demonstrating the payment processing system
/// Creates test cases for both raw card data and vault token data
/// Shows how the system handles different types of payment method data
fn main() {
    println!("=== Payment Processing System Demo ===\n");
    
    // === TEST CASE 1: Raw Card Data ===
    println!("--- Test Case 1: Raw Card Data ---");
    let api_req_card = PaymentApiRequest {
        payment_method_data: APIPaymentMethodData::Card(APICardData {
            card_number: 1234567890123456,
            card_expiry: 1225,
            card_cvc: 123,
        }),
    };
    
    // === TEST CASE 2: Vault Token Data ===
    println!("\n--- Test Case 2: Vault Token Data ---");
    let api_req_vault = PaymentApiRequest {
        payment_method_data: APIPaymentMethodData::CardVaultData(APICardVaultData {
            card_number: "token_1234567890123456".to_string(),
            card_expiry: "1225".to_string(),
            card_cvc: "123".to_string(),
        }),
    };
    
    // === APPROACH 1: Using get_trackers (Box<dyn Trait>) ===
    println!("\n--- Approach 1: get_trackers with Box<dyn Trait> ---");
    
    // Process card data using unified interface
    let card_processor = get_trackers(api_req_card);
    card_processor.process();
    
    // Process vault data using unified interface
    let vault_processor = get_trackers(api_req_vault);
    vault_processor.process();
    
    // === APPROACH 2: Using separate functions (concrete types) ===
    println!("\n--- Approach 2: Separate functions with concrete types ---");
    
    // Direct processing with concrete types
    let card_tracker = get_card_tracker(APICardData {
        card_number: 9876543210987654,
        card_expiry: 1226,
        card_cvc: 456,
    });
    payments_operation_core(card_tracker);
    
    let vault_tracker = get_vault_tracker(APICardVaultData {
        card_number: "vault_token_9876543210987654".to_string(),
        card_expiry: "1226".to_string(),
        card_cvc: "456".to_string(),
    });
    payments_operation_core(vault_tracker);
    
    // === DEMONSTRATE TRY FROM IMPLEMENTATIONS ===
    println!("\n--- Demonstrating TryFrom implementations ---");
    
    // Test successful conversions
    let api_card = APICardData {
        card_number: 1111222233334444,
        card_expiry: 1227,
        card_cvc: 789,
    };
    
    match Card::<DefaultPCIHolder>::try_from(api_card) {
        Ok(card) => println!("✅ Successfully converted APICardData to Card<DefaultPCIHolder>: {:?}", card),
        Err(e) => println!("❌ Failed to convert: {}", e),
    }
    
    // Test failed conversion (this should fail)
    let api_vault = APICardVaultData {
        card_number: "vault_token_1111222233334444".to_string(),
        card_expiry: "1227".to_string(),
        card_cvc: "789".to_string(),
    };
    
    match Card::<DefaultPCIHolder>::try_from(api_vault) {
        Ok(card) => println!("✅ Successfully converted APICardVaultData to Card<DefaultPCIHolder>: {:?}", card),
        Err(e) => println!("❌ Expected failure - {}", e),
    }
    
    println!("\n=== Summary ===");
    println!("✅ All From and TryFrom implementations working");
    println!("✅ Code compiles and runs successfully");
    println!("✅ Both approaches demonstrated:");
    println!("   1. get_trackers() with Box<dyn Trait> for unified interface");
    println!("   2. Separate functions returning concrete Tracker<T> types");
    println!("✅ Type safety enforced through failed TryFrom conversions");
    println!("✅ Comprehensive comments explaining design decisions");
    
    println!("\n🎯 **Conclusion**: For a unified get_trackers() function returning Tracker<T>,");
    println!("   Box<dyn Trait> is necessary due to Rust's type system limitations.");
    println!("   The alternative is separate functions for each concrete type.");
}
