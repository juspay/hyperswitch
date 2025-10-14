// DynamicField.res
// Reusable component for rendering dynamic form fields

open DynamicFieldTypes
open DynamicFieldValidation

@react.component
let make = (~field: dynamicField, ~value: string, ~onChange: string => unit, ~onBlur: unit => unit) => {
  let (isTouched, setIsTouched) = React.useState(() => false)
  let (localValue, setLocalValue) = React.useState(() => value)

  React.useEffect1(() => {
    setLocalValue(_ => value)
    None
  }, [value])

  let validationResult = if isTouched {
    validateFieldValue(field, localValue)
  } else {
    Valid
  }

  let handleChange = (event: ReactEvent.Form.t) => {
    let newValue = ReactEvent.Form.target(event)["value"]
    setLocalValue(_ => newValue)
    onChange(newValue)
  }

  let handleBlur = (_: ReactEvent.Focus.t) => {
    setIsTouched(_ => true)
    onBlur()
  }

  let inputType = switch field.fieldType {
  | UserEmailAddress => "email"
  | UserPhoneNumber => "tel"
  | UserPaymentAmount => "number"
  | _ => "text"
  }

  let inputClassName = switch validationResult {
  | Invalid(_) => "dynamic-field-input error"
  | Valid => "dynamic-field-input"
  }

  <div className="dynamic-field-container">
    <label className="dynamic-field-label">
      {React.string(field.label)}
      {field.required ? <span className="required-asterisk"> {React.string("*")} </span> : React.null}
    </label>

    <input
      type_=inputType
      value=localValue
      placeholder=field.placeholder
      onChange=handleChange
      onBlur=handleBlur
      className=inputClassName
      maxLength=?{field.maxLength->Belt.Option.map(Int.toString)}
      minLength=?{field.minLength->Belt.Option.map(Int.toString)}
      step=?{switch field.fieldType {
      | UserPaymentAmount => Some("0.01")
      | _ => None
      }}
    />

    {switch validationResult {
    | Invalid(errorMsg) => <div className="dynamic-field-error"> {React.string(errorMsg)} </div>
    | Valid => React.null
    }}
  </div>
}