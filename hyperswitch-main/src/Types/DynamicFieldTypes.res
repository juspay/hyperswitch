// DynamicFieldTypes.res
// Type definitions for dynamic field rendering in Hyperswitch WebSDK

type fieldType =
  | UserFullName
  | UserEmailAddress
  | UserPhoneNumber
  | UserAddressLine1
  | UserAddressLine2
  | UserCity
  | UserState
  | UserCountry
  | UserZipCode
  | UserBankAccountNumber
  | UserBankCode
  | UserBankAccountHolderName
  | UserMandiriVaNumber
  | UserPaymentAmount
  | UserPaymentCurrency
  | UserPaymentDescription
  | UserPaymentReference
  | UserCardNumber
  | UserCardCvc

type dynamicField = {
  requiredField: string,
  displayName: string,
  fieldType: fieldType,
  value: option<string>,
  required: bool,
  placeholder: option<string>,
  validation: option<string>,
  options: option<array<string>>, // For select dropdowns
}

type dynamicFieldConfig = {
  paymentMethod: string,
  paymentMethodType: string,
  requiredFields: Js.Dict.t<dynamicField>,
}

type fieldValidation = {
  isValid: bool,
  errorMessage: option<string>,
}

let fieldTypeToString = (fieldType: fieldType): string => {
  switch fieldType {
  | UserFullName => "user_full_name"
  | UserEmailAddress => "user_email_address"
  | UserPhoneNumber => "user_phone_number"
  | UserAddressLine1 => "user_address_line1"
  | UserAddressLine2 => "user_address_line2"
  | UserCity => "user_city"
  | UserState => "user_state"
  | UserCountry => "user_country"
  | UserZipCode => "user_zip_code"
  | UserBankAccountNumber => "user_bank_account_number"
  | UserBankCode => "user_bank_code"
  | UserBankAccountHolderName => "user_bank_account_holder_name"
  | UserMandiriVaNumber => "user_mandiri_va_number"
  | UserPaymentAmount => "user_payment_amount"
  | UserPaymentCurrency => "user_payment_currency"
  | UserPaymentDescription => "user_payment_description"
  | UserPaymentReference => "user_payment_reference"
  | UserCardNumber => "user_card_number"
  | UserCardCvc => "user_card_cvc"
  }
}

let stringToFieldType = (str: string): option<fieldType> => {
  switch str {
  | "user_full_name" => Some(UserFullName)
  | "user_email_address" => Some(UserEmailAddress)
  | "user_phone_number" => Some(UserPhoneNumber)
  | "user_address_line1" => Some(UserAddressLine1)
  | "user_address_line2" => Some(UserAddressLine2)
  | "user_city" => Some(UserCity)
  | "user_state" => Some(UserState)
  | "user_country" => Some(UserCountry)
  | "user_zip_code" => Some(UserZipCode)
  | "user_bank_account_number" => Some(UserBankAccountNumber)
  | "user_bank_code" => Some(UserBankCode)
  | "user_bank_account_holder_name" => Some(UserBankAccountHolderName)
  | "user_mandiri_va_number" => Some(UserMandiriVaNumber)
  | "user_payment_amount" => Some(UserPaymentAmount)
  | "user_payment_currency" => Some(UserPaymentCurrency)
  | "user_payment_description" => Some(UserPaymentDescription)
  | "user_payment_reference" => Some(UserPaymentReference)
  | "user_card_number" => Some(UserCardNumber)
  | "user_card_cvc" => Some(UserCardCvc)
  | _ => None
  }
}