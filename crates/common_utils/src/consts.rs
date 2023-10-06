//! Commonly used constants

/// Number of characters in a generated ID
pub const ID_LENGTH: usize = 20;

/// Characters to use for generating NanoID
pub(crate) const ALPHABETS: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B',
    'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z',
];

/// TTL for token
pub const TOKEN_TTL: i64 = 900;

///an example of the frm_configs json
pub static FRM_CONFIGS_EG: &str = r#"
[{"gateway":"stripe","payment_methods":[{"payment_method":"card","payment_method_types":[{"payment_method_type":"credit","card_networks":["Visa"],"flow":"pre","action":"cancel_txn"},{"payment_method_type":"debit","card_networks":["Visa"],"flow":"pre"}]}]}]
"#;

/// Maximum limit for payments list get api
pub const PAYMENTS_LIST_MAX_LIMIT_V1: u32 = 100;
/// Maximum limit for payments list post api with filters
pub const PAYMENTS_LIST_MAX_LIMIT_V2: u32 = 20;

/// surcharge percentage maximum precision length
pub const SURCHARGE_PERCENTAGE_PRECISION_LENGTH: u8 = 2;
