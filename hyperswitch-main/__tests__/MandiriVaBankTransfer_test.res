// __tests__/MandiriVaBankTransfer_test.res
// Integration tests for MandiriVaBankTransfer component

open Jest
open Expect
open ReactTestingLibrary

describe("MandiriVaBankTransfer", () => {
  test("should render all required fields", () => {
    let onSuccess = Jest.fn(() => ())
    let onError = Jest.fn(() => ())

    let component = <MandiriVaBankTransfer onPaymentSuccess=onSuccess onPaymentError=onError />

    let {getByText, getByPlaceholderText} = render(component)

    // Check title and description
    expect(getByText("Mandiri VA Bank Transfer"))->toBeInTheDocument
    expect(getByText("Complete your payment using Mandiri Virtual Account. You'll receive a VA number to make the transfer."))->toBeInTheDocument

    // Check all required fields are present
    expect(getByText("Full Name*"))->toBeInTheDocument
    expect(getByPlaceholderText("Enter your full name"))->toBeInTheDocument

    expect(getByText("Email Address*"))->toBeInTheDocument
    expect(getByPlaceholderText("Enter your email address"))->toBeInTheDocument

    expect(getByText("Phone Number*"))->toBeInTheDocument
    expect(getByPlaceholderText("Enter your phone number"))->toBeInTheDocument

    expect(getByText("Mandiri VA Number*"))->toBeInTheDocument
    expect(getByPlaceholderText("Enter your Mandiri VA number"))->toBeInTheDocument

    expect(getByText("Payment Amount*"))->toBeInTheDocument
    expect(getByPlaceholderText("Enter payment amount"))->toBeInTheDocument

    expect(getByText("Currency*"))->toBeInTheDocument
    expect(getByPlaceholderText("IDR"))->toBeInTheDocument

    expect(getByText("Payment Description"))->toBeInTheDocument
    expect(getByPlaceholderText("Enter payment description"))->toBeInTheDocument

    // Check submit button
    expect(getByText("Pay with Mandiri VA"))->toBeInTheDocument
  })

  test("should show validation errors for invalid inputs", () => {
    let onSuccess = Jest.fn(() => ())
    let onError = Jest.fn(() => ())

    let component = <MandiriVaBankTransfer onPaymentSuccess=onSuccess onPaymentError=onError />

    let {getByPlaceholderText, getByText} = render(component)

    // Fill in invalid email
    let emailInput = getByPlaceholderText("Enter your email address")
    fireEvent.change(emailInput, {"target": {"value": "invalid-email"}})
    fireEvent.blur(emailInput)

    // Should show validation error
    expect(getByText("Please enter a valid email address"))->toBeInTheDocument
  })

  test("should submit form with valid data", () => {
    let onSuccess = Jest.fn(() => ())
    let onError = Jest.fn(() => ())

    let component = <MandiriVaBankTransfer onPaymentSuccess=onSuccess onPaymentError=onError />

    let {getByPlaceholderText, getByText} = render(component)

    // Fill in all required fields with valid data
    fireEvent.change(getByPlaceholderText("Enter your full name"), {"target": {"value": "John Doe"}})
    fireEvent.change(getByPlaceholderText("Enter your email address"), {"target": {"value": "john@example.com"}})
    fireEvent.change(getByPlaceholderText("Enter your phone number"), {"target": {"value": "+1234567890"}})
    fireEvent.change(getByPlaceholderText("Enter your Mandiri VA number"), {"target": {"value": "1234567890123456"}})
    fireEvent.change(getByPlaceholderText("Enter payment amount"), {"target": {"value": "100.50"}})
    fireEvent.change(getByPlaceholderText("IDR"), {"target": {"value": "IDR"}})
    fireEvent.change(getByPlaceholderText("Enter payment description"), {"target": {"value": "Test payment"}})

    // Submit the form
    let submitButton = getByText("Pay with Mandiri VA")
    fireEvent.click(submitButton)

    // Should call onPaymentSuccess with payment data
    expect(onSuccess)->toHaveBeenCalledTimes(1)
    let callArgs = Jest.mocked(onSuccess).mock.calls[0][0]

    // Verify the payment data structure
    expect(Js.Json.decodeObject(callArgs))->toEqual(Some(Js.Dict.fromArray([
      ("user_full_name", Js.Json.string("John Doe")),
      ("user_email_address", Js.Json.string("john@example.com")),
      ("user_phone_number", Js.Json.string("+1234567890")),
      ("user_mandiri_va_number", Js.Json.string("1234567890123456")),
      ("user_payment_amount", Js.Json.string("100.50")),
      ("user_payment_currency", Js.Json.string("IDR")),
      ("user_payment_description", Js.Json.string("Test payment")),
      ("payment_method", Js.Json.string("bank_transfer")),
      ("payment_method_type", Js.Json.string("mandiri_va")),
    ])))
  })

  test("should not submit form with invalid data", () => {
    let onSuccess = Jest.fn(() => ())
    let onError = Jest.fn(() => ())

    let component = <MandiriVaBankTransfer onPaymentSuccess=onSuccess onPaymentError=onError />

    let {getByPlaceholderText, getByText} = render(component)

    // Fill in some fields but leave required ones empty
    fireEvent.change(getByPlaceholderText("Enter your full name"), {"target": {"value": ""}})
    fireEvent.change(getByPlaceholderText("Enter your email address"), {"target": {"value": "invalid-email"}})

    // Submit the form
    let submitButton = getByText("Pay with Mandiri VA")
    fireEvent.click(submitButton)

    // Should not call onPaymentSuccess
    expect(onSuccess)->not->toHaveBeenCalled
  })

  test("should handle field changes correctly", () => {
    let onSuccess = Jest.fn(() => ())
    let onError = Jest.fn(() => ())

    let component = <MandiriVaBankTransfer onPaymentSuccess=onSuccess onPaymentError=onError />

    let {getByPlaceholderText} = render(component)

    let nameInput = getByPlaceholderText("Enter your full name")

    // Change the input value
    fireEvent.change(nameInput, {"target": {"value": "Jane Smith"}})

    // Verify the value was updated
    expect(nameInput)->toHaveValue("Jane Smith")
  })
})