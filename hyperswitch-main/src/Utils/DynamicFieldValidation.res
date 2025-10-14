// DynamicFieldValidation.res
// Validation utilities for dynamic fields

open DynamicFieldTypes

// Regular expressions for validation
let emailRegex = %re("/^[^\s@]+@[^\s@]+\.[^\s@]+$/")
let phoneRegex = %re("/^\+?[1-9]\d{1,14}$/")
let nameRegex = %re("/^[a-zA-Z\s'-]+$/")
let numericRegex = %re("/^\d+$/")
let alphaNumericRegex = %re("/^[a-zA-Z0-9]+$/")
let cardNumberRegex = %re("/^\d{13,19}$/")
let cardCvcRegex = %re("/^\d{3,4}$/")

let validateFieldValue = (field: dynamicField, value: string): fieldValidation => {
  // Check if required field is empty
  if field.required && String.trim(value) === "" {
    {isValid: false, errorMessage: Some("This field is required")}
  } else if !field.required && String.trim(value) === "" {
    {isValid: true, errorMessage: None}
  } else {
    // Check regex validation if provided
    switch field.validation {
    | Some(regexStr) =>
      let regex = try Js.Re.fromString(regexStr) catch {
      | _ => Js.Re.fromString(".*") // Fallback to match anything if regex is invalid
      }
      if !Js.Re.test_(regex, value) {
        {isValid: false, errorMessage: Some("Invalid format")}
      } else {
        {isValid: true, errorMessage: None}
      }
    | None =>
      // Default validation based on field type
      switch field.fieldType {
      | UserEmailAddress =>
        if Js.Re.test_(emailRegex, value) {
          {isValid: true, errorMessage: None}
        } else {
          {isValid: false, errorMessage: Some("Please enter a valid email address")}
        }
      | UserPhoneNumber =>
        if Js.Re.test_(phoneRegex, value) {
          {isValid: true, errorMessage: None}
        } else {
          {isValid: false, errorMessage: Some("Please enter a valid phone number")}
        }
      | UserFullName =>
        if Js.Re.test_(nameRegex, value) {
          {isValid: true, errorMessage: None}
        } else {
          {isValid: false, errorMessage: Some("Please enter a valid name (letters, spaces, hyphens, and apostrophes only)")}
        }
      | UserBankAccountNumber =>
        if Js.Re.test_(numericRegex, value) {
          {isValid: true, errorMessage: None}
        } else {
          {isValid: false, errorMessage: Some("Please enter a valid account number (numbers only)")}
        }
      | UserBankCode =>
        if Js.Re.test_(alphaNumericRegex, value) {
          {isValid: true, errorMessage: None}
        } else {
          {isValid: false, errorMessage: Some("Please enter a valid bank code (letters and numbers only)")}
        }
      | UserBankAccountHolderName =>
        if Js.Re.test_(nameRegex, value) {
          {isValid: true, errorMessage: None}
        } else {
          {isValid: false, errorMessage: Some("Please enter a valid account holder name")}
        }
      | UserMandiriVaNumber =>
        if Js.Re.test_(numericRegex, value) && String.length(value) >= 10 && String.length(value) <= 16 {
          {isValid: true, errorMessage: None}
        } else {
          {isValid: false, errorMessage: Some("Please enter a valid Mandiri VA number (10-16 digits)")}
        }
      | UserCardNumber =>
        if Js.Re.test_(cardNumberRegex, value) {
          {isValid: true, errorMessage: None}
        } else {
          {isValid: false, errorMessage: Some("Please enter a valid card number (13-19 digits)")}
        }
      | UserCardCvc =>
        if Js.Re.test_(cardCvcRegex, value) {
          {isValid: true, errorMessage: None}
        } else {
          {isValid: false, errorMessage: Some("Please enter a valid CVC (3-4 digits)")}
        }
      | UserPaymentAmount =>
        switch Float.fromString(value) {
        | Some(amount) if amount > 0.0 => {isValid: true, errorMessage: None}
        | _ => {isValid: false, errorMessage: Some("Please enter a valid payment amount")}
        }
      | UserPaymentCurrency =>
        if String.length(value) === 3 && Js.Re.test_(%re("/^[A-Z]+$/"), value) {
          {isValid: true, errorMessage: None}
        } else {
          {isValid: false, errorMessage: Some("Please enter a valid 3-letter currency code")}
        }
      | _ => {isValid: true, errorMessage: None} // Default validation for other field types
      }
    }
  }
}

let getFieldTypeFromString = (str: string): option<fieldType> => {
  stringToFieldType(str)
}