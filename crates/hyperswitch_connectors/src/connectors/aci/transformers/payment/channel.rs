//! The channel a payment came in through, and what it changes.

use common_enums::enums;

use crate::connectors::aci::transformers::{
    metadata::AciMetadata,
    request::{AciTransactionCategory, Instruction},
};

/// OPPWA's `transactionCategory`, resolved from the two places that can declare it.
///
/// Without it, a mail or telephone order is submitted indistinguishably from e-commerce, which
/// misstates SCA scope (MOTO is out of scope for PSD2 SCA) and can misqualify interchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::connectors::aci::transformers) enum Channel {
    /// Neither the account nor the request named a channel OPPWA has a code for:
    /// `transactionCategory` is omitted.
    Undeclared,
    /// E-commerce, point of sale or mPOS.
    Standard(AciTransactionCategory),
    /// A mail or telephone order (`MO`, `TO` or `MOTO`).
    MailOrTelephoneOrder(AciTransactionCategory),
}

impl Channel {
    /// The channel, **MCA first**.
    ///
    /// An entity is boarded for a channel: a MOTO entity takes orders by phone and by nothing
    /// else, and that boarding is what the acquirer priced and what SCA scope follows from. A
    /// per-request `payment_channel` is set by whatever client built the request, which for a
    /// fixed-channel merchant is usually a checkout default that never knew the entity was MOTO.
    /// Letting the request win there would silently downgrade a MOTO entity to e-commerce on
    /// every payment, so the account-level configuration is authoritative and the request only
    /// fills the gap when the MCA says nothing.
    ///
    /// Note this inverts the precedence PT-3992 specified; PT-4026 reversed it deliberately.
    pub(in crate::connectors::aci::transformers) fn resolve(
        metadata: AciMetadata,
        payment_channel: Option<&enums::PaymentChannel>,
    ) -> Self {
        let from_request = || match payment_channel {
            Some(enums::PaymentChannel::Ecommerce) => Some(AciTransactionCategory::Ecommerce),
            Some(enums::PaymentChannel::MailOrder) => Some(AciTransactionCategory::MailOrder),
            Some(enums::PaymentChannel::TelephoneOrder) => {
                Some(AciTransactionCategory::TelephoneOrder)
            }
            // A free-form channel is not one of OPPWA's codes, and `None` means the request did
            // not say — omit the field, which preserves the pre-PT-3992 wire exactly.
            Some(enums::PaymentChannel::Other(_)) | None => None,
        };
        match metadata.transaction_category.or_else(from_request) {
            None => Self::Undeclared,
            Some(
                category @ (AciTransactionCategory::MailOrder
                | AciTransactionCategory::TelephoneOrder
                | AciTransactionCategory::MailOrTelephoneOrder),
            ) => Self::MailOrTelephoneOrder(category),
            // `PO`/`PM` are card-present: a different channel with none of the MOTO consequences.
            Some(
                category @ (AciTransactionCategory::Ecommerce
                | AciTransactionCategory::PointOfSale
                | AciTransactionCategory::MobilePointOfSale),
            ) => Self::Standard(category),
        }
    }

    /// The `transactionCategory` sent.
    pub(super) fn category(self) -> Option<AciTransactionCategory> {
        match self {
            Self::Undeclared => None,
            Self::Standard(category) | Self::MailOrTelephoneOrder(category) => Some(category),
        }
    }

    pub(in crate::connectors::aci::transformers) fn is_moto(self) -> bool {
        matches!(self, Self::MailOrTelephoneOrder(_))
    }

    /// What of a card-on-file `instruction` this channel sends.
    ///
    /// A MOTO order is submitted with `transactionCategory` and nothing else describing the
    /// agreement: the channel is the whole statement. `createRegistration` survives because it
    /// is a top-level OPPWA parameter rather than part of the block, and it is what actually
    /// stores the card — dropping it too would leave a vaulting request that vaults nothing,
    /// which is the failure the `/v1/registrations` path had by construction. With no
    /// registration to request, a MOTO leg sends no instruction fields at all.
    ///
    /// The consequence is deliberate and worth stating: no
    /// `standingInstruction.initialTransactionId` or `agreementId` is sent on a MOTO MIT, so the
    /// acquirer cannot match that charge to a registered series. Card-on-file linkage and MOTO
    /// are mutually exclusive on this connector.
    pub(super) fn declare(self, instruction: Instruction) -> Option<Instruction> {
        match self {
            Self::MailOrTelephoneOrder(_) => {
                instruction
                    .create_registration
                    .map(|create_registration| Instruction {
                        create_registration: Some(create_registration),
                        ..Instruction::default()
                    })
            }
            Self::Undeclared | Self::Standard(_) => Some(instruction),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(category: AciTransactionCategory) -> AciMetadata {
        AciMetadata {
            transaction_category: Some(category),
            ..AciMetadata::default()
        }
    }

    /// The account-level channel wins over the request's. A MOTO entity is boarded for one
    /// channel, and a checkout default in the request must not be able to downgrade it to
    /// e-commerce; the request only fills the gap when the MCA says nothing.
    #[test]
    fn mca_transaction_category_takes_precedence_over_payment_channel() {
        assert_eq!(
            Channel::resolve(
                declared(AciTransactionCategory::MailOrTelephoneOrder),
                Some(&enums::PaymentChannel::Ecommerce)
            ),
            Channel::MailOrTelephoneOrder(AciTransactionCategory::MailOrTelephoneOrder),
        );

        // No MCA value: the request decides.
        assert_eq!(
            Channel::resolve(
                AciMetadata::default(),
                Some(&enums::PaymentChannel::TelephoneOrder)
            ),
            Channel::MailOrTelephoneOrder(AciTransactionCategory::TelephoneOrder),
        );

        // Neither says anything, or the request names a channel OPPWA has no code for: the
        // field is omitted, which is the pre-existing wire.
        assert_eq!(
            Channel::resolve(AciMetadata::default(), None),
            Channel::Undeclared
        );
        assert_eq!(
            Channel::resolve(
                AciMetadata::default(),
                Some(&enums::PaymentChannel::Other("kiosk".to_string()))
            ),
            Channel::Undeclared,
        );
    }

    /// Only mail and telephone orders count. Card-present channels are a different thing
    /// entirely and keep their standing instructions.
    #[test]
    fn moto_channel_covers_mail_telephone_and_the_combined_code() {
        for category in [
            AciTransactionCategory::MailOrder,
            AciTransactionCategory::TelephoneOrder,
            AciTransactionCategory::MailOrTelephoneOrder,
        ] {
            let channel = Channel::resolve(declared(category), None);
            assert!(channel.is_moto(), "{category:?}");
            assert_eq!(channel.category(), Some(category));
        }

        for category in [
            AciTransactionCategory::Ecommerce,
            AciTransactionCategory::PointOfSale,
            AciTransactionCategory::MobilePointOfSale,
        ] {
            let channel = Channel::resolve(declared(category), None);
            assert!(!channel.is_moto(), "{category:?}");
            assert_eq!(channel.category(), Some(category));
        }

        assert!(!Channel::Undeclared.is_moto());
    }
}
