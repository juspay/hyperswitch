// __tests__/DynamicFieldValidation_test.res
// Unit tests for DynamicFieldValidation

open Jest
open Expect
open DynamicFieldValidation
open DynamicFieldTypes

describe("DynamicFieldValidation", () => {
  describe("validateFieldValue", () => {
    test("should validate required field with empty value", () => {
      let field = {
        fieldType: UserFullName,
        label: "Full Name",
        placeholder: "Enter name",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: None,
      }

      let result = validateFieldValue(field, "")
      expect(result)->toEqual(Invalid("This field is required"))
    })

    test("should validate required field with whitespace only", () => {
      let field = {
        fieldType: UserFullName,
        label: "Full Name",
        placeholder: "Enter name",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: None,
      }

      let result = validateFieldValue(field, "   ")
      expect(result)->toEqual(Invalid("This field is required"))
    })

    test("should pass validation for optional empty field", () => {
      let field = {
        fieldType: UserFullName,
        label: "Full Name",
        placeholder: "Enter name",
        required: false,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: None,
      }

      let result = validateFieldValue(field, "")
      expect(result)->toEqual(Valid)
    })

    test("should validate email format", () => {
      let field = {
        fieldType: UserEmailAddress,
        label: "Email",
        placeholder: "Enter email",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: None,
      }

      expect(validateFieldValue(field, "test@example.com"))->toEqual(Valid)
      expect(validateFieldValue(field, "invalid-email"))->toEqual(Invalid("Please enter a valid email address"))
    })

    test("should validate phone number format", () => {
      let field = {
        fieldType: UserPhoneNumber,
        label: "Phone",
        placeholder: "Enter phone",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: None,
      }

      expect(validateFieldValue(field, "+1234567890"))->toEqual(Valid)
      expect(validateFieldValue(field, "123-456-7890"))->toEqual(Invalid("Please enter a valid phone number"))
    })

    test("should validate name format", () => {
      let field = {
        fieldType: UserFullName,
        label: "Name",
        placeholder: "Enter name",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: None,
      }

      expect(validateFieldValue(field, "John Doe"))->toEqual(Valid)
      expect(validateFieldValue(field, "John123"))->toEqual(Invalid("Please enter a valid name (letters, spaces, hyphens, and apostrophes only)"))
    })

    test("should validate bank account number", () => {
      let field = {
        fieldType: UserBankAccountNumber,
        label: "Account Number",
        placeholder: "Enter account number",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: None,
      }

      expect(validateFieldValue(field, "1234567890"))->toEqual(Valid)
      expect(validateFieldValue(field, "123abc"))->toEqual(Invalid("Please enter a valid account number (numbers only)"))
    })

    test("should validate Mandiri VA number length", () => {
      let field = {
        fieldType: UserMandiriVaNumber,
        label: "VA Number",
        placeholder: "Enter VA number",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: None,
      }

      expect(validateFieldValue(field, "1234567890123456"))->toEqual(Valid) // 16 digits
      expect(validateFieldValue(field, "123456789"))->toEqual(Invalid("Please enter a valid Mandiri VA number (10-16 digits)")) // 9 digits
      expect(validateFieldValue(field, "12345678901234567890"))->toEqual(Invalid("Please enter a valid Mandiri VA number (10-16 digits)")) // 20 digits
    })

    test("should validate payment amount", () => {
      let field = {
        fieldType: UserPaymentAmount,
        label: "Amount",
        placeholder: "Enter amount",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: None,
      }

      expect(validateFieldValue(field, "100.50"))->toEqual(Valid)
      expect(validateFieldValue(field, "0"))->toEqual(Invalid("Please enter a valid payment amount"))
      expect(validateFieldValue(field, "-50"))->toEqual(Invalid("Please enter a valid payment amount"))
    })

    test("should validate currency code", () => {
      let field = {
        fieldType: UserPaymentCurrency,
        label: "Currency",
        placeholder: "Enter currency",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: None,
      }

      expect(validateFieldValue(field, "IDR"))->toEqual(Valid)
      expect(validateFieldValue(field, "USD"))->toEqual(Valid)
      expect(validateFieldValue(field, "US"))->toEqual(Invalid("Please enter a valid 3-letter currency code"))
      expect(validateFieldValue(field, "us1"))->toEqual(Invalid("Please enter a valid 3-letter currency code"))
    })

    test("should validate minimum length", () => {
      let field = {
        fieldType: UserFullName,
        label: "Name",
        placeholder: "Enter name",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: None,
        minLength: Some(3),
      }

      expect(validateFieldValue(field, "John"))->toEqual(Valid)
      expect(validateFieldValue(field, "Jo"))->toEqual(Invalid("Minimum length is 3 characters"))
    })

    test("should validate maximum length", () => {
      let field = {
        fieldType: UserFullName,
        label: "Name",
        placeholder: "Enter name",
        required: true,
        validationRegex: None,
        errorMessage: None,
        maxLength: Some(10),
        minLength: None,
      }

      expect(validateFieldValue(field, "John"))->toEqual(Valid)
      expect(validateFieldValue(field, "This is a very long name"))->toEqual(Invalid("Maximum length is 10 characters"))
    })
  })

  describe("getFieldTypeFromString", () => {
    test("should convert string to field type", () => {
      expect(getFieldTypeFromString("user_full_name"))->toEqual(Some(UserFullName))
      expect(getFieldTypeFromString("user_email_address"))->toEqual(Some(UserEmailAddress))
      expect(getFieldTypeFromString("invalid_field"))->toEqual(None)
    })
  })
})