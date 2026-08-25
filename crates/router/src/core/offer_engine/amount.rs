use common_utils::types::{AmountConvertor, MinorUnit, StringMajorUnit, StringMajorUnitForCore};
use error_stack::ResultExt;
use rust_decimal::{prelude::ToPrimitive, Decimal};

use super::types::OfferEngineError;

/// Convert Hyperswitch minor units to an Offer Engine decimal string.
pub fn to_offer_engine_amount(
    amount: MinorUnit,
    currency: common_enums::Currency,
) -> error_stack::Result<StringMajorUnit, OfferEngineError> {
    StringMajorUnitForCore
        .convert(amount, currency)
        .change_context(OfferEngineError::AmountConversion)
        .attach_printable("Failed to convert minor units to Offer Engine amount string")
}

/// Convert an Offer Engine decimal string into Hyperswitch minor units.
pub fn from_offer_engine_amount(
    amount: &StringMajorUnit,
    currency: common_enums::Currency,
) -> error_stack::Result<MinorUnit, OfferEngineError> {
    let decimal = Decimal::from_str_exact(&amount.get_amount_as_string())
        .change_context(OfferEngineError::AmountConversion)
        .attach_printable("Offer Engine amount is not a valid decimal")?;

    common_utils::fp_utils::when(decimal.is_sign_negative(), || {
        Err(error_stack::report!(OfferEngineError::AmountConversion)
            .attach_printable("Offer Engine amount is negative"))
    })?;

    let decimals = u32::from(currency.number_of_digits_after_decimal_point());
    let factor = Decimal::from(10_u64.pow(decimals));
    let scaled = decimal * factor;

    common_utils::fp_utils::when(scaled.fract() != Decimal::ZERO, || {
        Err(
            error_stack::report!(OfferEngineError::AmountConversion).attach_printable(
                "Offer Engine amount has more decimal places than the currency allows",
            ),
        )
    })?;

    let minor = scaled
        .to_i64()
        .ok_or(OfferEngineError::AmountConversion)
        .attach_printable("Offer Engine amount does not fit in minor units")?;

    Ok(MinorUnit::new(minor))
}
