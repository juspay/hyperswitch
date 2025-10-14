// DynamicFieldTypes.res// Field types that can be rendered dynamically

// Type definitions for dynamic field rendering in Hyperswitch WebSDKtype fieldType = 

  | UserFullName

type fieldType =  | UserEmailAddress  

  | UserFullName  | UserPhoneNumber

  | UserEmailAddress  | UserAddress

  | UserPhoneNumber  | UserCountry

  | UserAddressLine1  | UserText

  | UserAddressLine2  | UserNumber

  | UserCity  | UserSelect

  | UserState  | UserCheckbox

  | UserCountry  | UserDate

  | UserZipCode

  | UserBankAccountNumber// Individual field configuration

  | UserBankCodetype dynamicField = {

  | UserBankAccountHolderName  requiredField: string,

  | UserMandiriVaNumber  displayName: string,

  | UserPaymentAmount  fieldType: fieldType,

  | UserPaymentCurrency  value: option<string>,

  | UserPaymentDescription  required: bool,

  | UserPaymentReference  placeholder: option<string>,

  validation: option<string>,

type dynamicField = {  options: option<array<string>>, // For select dropdowns

  fieldType: fieldType,}

  label: string,

  placeholder: string,// Payment method dynamic configuration

  required: bool,type dynamicFieldConfig = {

  validationRegex: option<string>,  paymentMethod: string,

  errorMessage: option<string>,  paymentMethodType: string,

  maxLength: option<int>,  requiredFields: Dict.t<dynamicField>,

  minLength: option<int>,}

}

// Field validation result

type dynamicFieldConfig = {type fieldValidation = {

  fields: array<dynamicField>,  isValid: bool,

  submitButtonText: string,  errorMessage: option<string>,

  onSubmit: array<(string, string)> => unit, // Array of (fieldType, value) tuples}

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
  | _ => None
  }
}