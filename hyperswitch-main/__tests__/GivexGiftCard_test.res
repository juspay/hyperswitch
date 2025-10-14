// __tests__/GivexGiftCard_test.res
// Integration tests for GivexGiftCard component

open Jest
open Expect
open ReactTestingLibrary

describe("GivexGiftCard", () => {
  let mockConfig: DynamicFieldTypes.dynamicFieldConfig = {
    paymentMethod: "gift_card",
    paymentMethodType: "givex",
    requiredFields: Js.Dict.fromArray([
      ("payment_method_data.gift_card.number", {
        requiredField: "payment_method_data.gift_card.number",
        displayName: "Gift Card Number",
        fieldType: DynamicFieldTypes.UserCardNumber,
        value: None,
        required: true,
        placeholder: Some("Enter your gift card number"),
        validation: None,
        options: None,
      }),
      ("payment_method_data.gift_card.cvc", {
        requiredField: "payment_method_data.gift_card.cvc",
        displayName: "CVC",
        fieldType: DynamicFieldTypes.UserCardCvc,
        value: None,
        required: true,
        placeholder: Some("Enter CVC"),
        validation: None,
        options: None,
      }),
    ]),
  }

  test("should render all required fields", () => {
    let onSuccess = Jest.fn(() => ())
    let onError = Jest.fn(() => ())

    let component = <GivexGiftCard config=mockConfig onPaymentSuccess=onSuccess onPaymentError=onError />

    let {getByText, getByPlaceholderText} = render(component)

    // Check title and description
    expect(getByText("Givex Gift Card"))->toBeInTheDocument
    expect(getByText("Complete your payment using Givex Gift Card. Enter your card details below."))->toBeInTheDocument

    // Check all required fields are present
    expect(getByText("Gift Card Number*"))->toBeInTheDocument
    expect(getByPlaceholderText("Enter your gift card number"))->toBeInTheDocument

    expect(getByText("CVC*"))->toBeInTheDocument
    expect(getByPlaceholderText("Enter CVC"))->toBeInTheDocument

    // Check submit button
    expect(getByText("Pay with Givex"))->toBeInTheDocument
  })

  test("should show validation errors for invalid inputs", () => {
    let onSuccess = Jest.fn(() => ())
    let onError = Jest.fn(() => ())

    let component = <GivexGiftCard config=mockConfig onPaymentSuccess=onSuccess onPaymentError=onError />

    let {getByPlaceholderText, getByText} = render(component)

    // Fill in invalid card number
    let cardNumberInput = getByPlaceholderText("Enter your gift card number")
    fireEvent.change(cardNumberInput, {"target": {"value": "123"}})
    fireEvent.blur(cardNumberInput)

    // Fill in invalid CVC
    let cvcInput = getByPlaceholderText("Enter CVC")
    fireEvent.change(cvcInput, {"target": {"value": "12"}})
    fireEvent.blur(cvcInput)

    // Submit the form
    let submitButton = getByText("Pay with Givex")
    fireEvent.click(submitButton)

    // Should call onPaymentError with validation errors
    expect(onError)->toHaveBeenCalledTimes(1)
    let errorCall = Jest.mocked(onError).mock.calls[0][0]
    expect(errorCall)->toContain("Gift Card Number")
    expect(errorCall)->toContain("CVC")
  })

  test("should submit form with valid data", () => {
    let onSuccess = Jest.fn(() => ())
    let onError = Jest.fn(() => ())

    let component = <GivexGiftCard config=mockConfig onPaymentSuccess=onSuccess onPaymentError=onError />

    let {getByPlaceholderText, getByText} = render(component)

    // Fill in valid data
    fireEvent.change(getByPlaceholderText("Enter your gift card number"), {"target": {"value": "6364530000000000"}})
    fireEvent.change(getByPlaceholderText("Enter CVC"), {"target": {"value": "737"}})

    // Submit the form
    let submitButton = getByText("Pay with Givex")
    fireEvent.click(submitButton)

    // Should call onPaymentSuccess with payment data
    expect(onSuccess)->toHaveBeenCalledTimes(1)
    let callArgs = Jest.mocked(onSuccess).mock.calls[0][0]

    // Verify the payment data structure
    expect(Js.Json.decodeObject(callArgs))->toEqual(Some(Js.Dict.fromArray([
      ("payment_method", Js.Json.string("gift_card")),
      ("payment_method_type", Js.Json.string("givex")),
      ("payment_method_data", Js.Json.object_(Js.Dict.fromArray([
        ("gift_card", Js.Json.object_(Js.Dict.fromArray([
          ("givex", Js.Json.object_(Js.Dict.fromArray([
            ("number", Js.Json.string("6364530000000000")),
            ("cvc", Js.Json.string("737")),
          ])))
        ])))
      ]))),
    ])))
  })

  test("should not submit form with missing required fields", () => {
    let onSuccess = Jest.fn(() => ())
    let onError = Jest.fn(() => ())

    let component = <GivexGiftCard config=mockConfig onPaymentSuccess=onSuccess onPaymentError=onError />

    let {getByText} = render(component)

    // Submit the form without filling any fields
    let submitButton = getByText("Pay with Givex")
    fireEvent.click(submitButton)

    // Should call onPaymentError
    expect(onError)->toHaveBeenCalledTimes(1)
    expect(onSuccess)->not->toHaveBeenCalled
  })

  test("should handle field changes correctly", () => {
    let onSuccess = Jest.fn(() => ())
    let onError = Jest.fn(() => ())

    let component = <GivexGiftCard config=mockConfig onPaymentSuccess=onSuccess onPaymentError=onError />

    let {getByPlaceholderText} = render(component)

    let cardNumberInput = getByPlaceholderText("Enter your gift card number")

    // Change the input value
    fireEvent.change(cardNumberInput, {"target": {"value": "4111111111111111"}})

    // Verify the value was updated
    expect(cardNumberInput)->toHaveValue("4111111111111111")
  })
})