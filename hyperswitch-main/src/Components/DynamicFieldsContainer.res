// DynamicFieldsContainer.res
// Container component for rendering multiple dynamic fields

open DynamicFieldTypes

@react.component
let make = (~config: dynamicFieldConfig, ~fieldValues: Js.Dict.t<string>, ~onFieldChange: (fieldType, string) => unit) => {
  let handleFieldChange = (fieldType: fieldType, value: string) => {
    onFieldChange(fieldType, value)
  }

  let handleSubmit = (event: ReactEvent.Form.t) => {
    ReactEvent.Form.preventDefault(event)

    // Collect all field values
    let fieldValuePairs = config.fields->Belt.Array.map(field => {
      let fieldTypeStr = fieldTypeToString(field.fieldType)
      let value = Js.Dict.get(fieldValues, fieldTypeStr)->Belt.Option.getWithDefault("")
      (fieldTypeStr, value)
    })

    config.onSubmit(fieldValuePairs)
  }

  <form className="dynamic-fields-form" onSubmit=handleSubmit>
    {config.fields
    ->Belt.Array.map(field => {
      let fieldTypeStr = fieldTypeToString(field.fieldType)
      let currentValue = Js.Dict.get(fieldValues, fieldTypeStr)->Belt.Option.getWithDefault("")

      <DynamicField
        key=fieldTypeStr
        field=field
        value=currentValue
        onChange={value => handleFieldChange(field.fieldType, value)}
        onBlur={() => ()}
      />
    })
    ->React.array}

    <button type_="submit" className="dynamic-fields-submit-btn">
      {React.string(config.submitButtonText)}
    </button>
  </form>
}