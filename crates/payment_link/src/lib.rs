pub mod css_generator;
pub mod js_generator;
pub mod meta_tags;
pub mod template_renderer;
pub mod types;

#[cfg(feature = "wasm")]
pub mod wasm;

pub use css_generator::get_css_script;
pub use js_generator::get_js_script;
pub use meta_tags::get_meta_tags_html;
pub use template_renderer::{build_payment_link_html, build_secure_payment_link_html, get_payment_link_status};
pub use types::{PaymentLinkFormData, PaymentLinkStatusData};
