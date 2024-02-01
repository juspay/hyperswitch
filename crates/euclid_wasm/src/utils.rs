use wasm_bindgen::prelude::*;

pub trait JsResultExt<T> {
    fn err_to_js(self) -> Result<T, JsValue>;
}

impl<T, E> JsResultExt<T> for Result<T, E>
where
    E: serde::Serialize,
{
        /// Converts a Result type to a Result type with a JsValue error.
    fn err_to_js(self) -> Result<T, JsValue> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(serde_wasm_bindgen::to_value(&e)?),
        }
    }
}
