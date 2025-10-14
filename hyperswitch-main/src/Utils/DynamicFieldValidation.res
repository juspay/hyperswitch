// DynamicFieldValidation.resopen DynamicFieldTypes

// Validation utilities for dynamic fields

let validateFieldValue = (fieldType: fieldType, value: string, required: bool): fieldValidation => {

open DynamicFieldTypes  if required && String.length(value) == 0 {

    {isValid: false, errorMessage: Some("This field is required")}

// Regular expressions for validation  } else {

let emailRegex = %re("/^[^\s@]+@[^\s@]+\.[^\s@]+$/")    switch fieldType {

let phoneRegex = %re("/^\+?[1-9]\d{1,14}$/")    | UserEmailAddress => {

let nameRegex = %re("/^[a-zA-Z\s'-]+$/")        let emailRegex = %re("/^[^\s@]+@[^\s@]+\.[^\s@]+$/")

let numericRegex = %re("/^\d+$/")        let isValidEmail = Js.Re.test_(emailRegex, value)

let alphaNumericRegex = %re("/^[a-zA-Z0-9]+$/")        {

          isValid: isValidEmail || String.length(value) == 0,

type validationResult =          errorMessage: isValidEmail || String.length(value) == 0 ? None : Some("Please enter a valid email address")

  | Valid        }

  | Invalid(string)      }

    | UserFullName => {

let validateFieldValue = (field: dynamicField, value: string): validationResult => {        let nameRegex = %re("/^[a-zA-Z\s]{2,}$/")

  // Check if required field is empty        let isValidName = Js.Re.test_(nameRegex, value)

  if field.required && String.trim(value) === "" {        {

    Invalid("This field is required")          isValid: isValidName || String.length(value) == 0,

  } else if !field.required && String.trim(value) === "" {          errorMessage: isValidName || String.length(value) == 0 ? None : Some("Please enter a valid name (minimum 2 characters)")

    Valid        }

  } else {      }

    // Check minimum length    | UserPhoneNumber => {

    switch field.minLength {        let phoneRegex = %re("/^\+?[\d\s\-\(\)]{10,}$/")

    | Some(minLen) if String.length(value) < minLen =>        let isValidPhone = Js.Re.test_(phoneRegex, value)

      Invalid(`Minimum length is ${Int.toString(minLen)} characters`)        {

    | _ => ()          isValid: isValidPhone || String.length(value) == 0,

    }          errorMessage: isValidPhone || String.length(value) == 0 ? None : Some("Please enter a valid phone number")

        }

    // Check maximum length      }

    switch field.maxLength {    | _ => {isValid: true, errorMessage: None}

    | Some(maxLen) if String.length(value) > maxLen =>    }

      Invalid(`Maximum length is ${Int.toString(maxLen)} characters`)  }

    | _ => ()}

    }

let getFieldTypeFromString = (fieldTypeStr: string): fieldType => {

    // Check regex validation  switch fieldTypeStr {

    switch field.validationRegex {  | "user_full_name" => UserFullName

    | Some(regexStr) =>  | "user_email_address" => UserEmailAddress

      let regex = try Js.Re.fromString(regexStr) catch {  | "user_phone_number" => UserPhoneNumber

      | _ => Js.Re.fromString(".*") // Fallback to match anything if regex is invalid  | "user_address" => UserAddress

      }  | "user_country" => UserCountry

      if !Js.Re.test_(regex, value) {  | "user_text" => UserText

        Invalid(field.errorMessage->Belt.Option.getWithDefault("Invalid format"))  | "user_number" => UserNumber

      } else {  | "user_select" => UserSelect

        Valid  | "user_checkbox" => UserCheckbox

      }  | "user_date" => UserDate

    | None =>  | _ => UserText // Default fallback

      // Default validation based on field type  }

      switch field.fieldType {}

      | UserEmailAddress =>
        if Js.Re.test_(emailRegex, value) {
          Valid
        } else {
          Invalid("Please enter a valid email address")
        }
      | UserPhoneNumber =>
        if Js.Re.test_(phoneRegex, value) {
          Valid
        } else {
          Invalid("Please enter a valid phone number")
        }
      | UserFullName =>
        if Js.Re.test_(nameRegex, value) {
          Valid
        } else {
          Invalid("Please enter a valid name (letters, spaces, hyphens, and apostrophes only)")
        }
      | UserBankAccountNumber =>
        if Js.Re.test_(numericRegex, value) {
          Valid
        } else {
          Invalid("Please enter a valid account number (numbers only)")
        }
      | UserBankCode =>
        if Js.Re.test_(alphaNumericRegex, value) {
          Valid
        } else {
          Invalid("Please enter a valid bank code (letters and numbers only)")
        }
      | UserBankAccountHolderName =>
        if Js.Re.test_(nameRegex, value) {
          Valid
        } else {
          Invalid("Please enter a valid account holder name")
        }
      | UserMandiriVaNumber =>
        if Js.Re.test_(numericRegex, value) && String.length(value) >= 10 && String.length(value) <= 16 {
          Valid
        } else {
          Invalid("Please enter a valid Mandiri VA number (10-16 digits)")
        }
      | UserPaymentAmount =>
        switch Float.fromString(value) {
        | Some(amount) if amount > 0.0 => Valid
        | _ => Invalid("Please enter a valid payment amount")
        }
      | UserPaymentCurrency =>
        if String.length(value) === 3 && Js.Re.test_(%re("/^[A-Z]+$/"), value) {
          Valid
        } else {
          Invalid("Please enter a valid 3-letter currency code")
        }
      | _ => Valid // Default validation for other field types
      }
    }
  }
}

let getFieldTypeFromString = (str: string): option<fieldType> => {
  stringToFieldType(str)
}