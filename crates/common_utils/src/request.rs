use masking::{Maskable, Secret};
#[cfg(feature = "logs")]
use router_env::logger;
use serde::{Deserialize, Serialize};

use crate::errors;

pub type Headers = std::collections::HashSet<(String, Maskable<String>)>;

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, strum::Display, strum::EnumString,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ContentType {
    Json,
    FormUrlEncoded,
    FormData,
    Xml,
}

/// Returns the default request headers as an array of tuples containing header names and values.
fn default_request_headers() -> [(String, Maskable<String>); 1] {
    use http::header;

    [(header::VIA.to_string(), "HyperSwitch".to_string().into())]
}

#[derive(Debug)]
pub struct Request {
    pub url: String,
    pub headers: Headers,
    pub method: Method,
    pub certificate: Option<String>,
    pub certificate_key: Option<String>,
    pub body: Option<RequestContent>,
}

impl std::fmt::Debug for RequestContent {
        /// Formats the request body type to a string representation based on the enum variant.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Json(_) => "JsonRequestBody",
            Self::FormUrlEncoded(_) => "FormUrlEncodedRequestBody",
            Self::FormData(_) => "FormDataRequestBody",
            Self::Xml(_) => "XmlRequestBody",
        })
    }
}

pub enum RequestContent {
    Json(Box<dyn masking::ErasedMaskSerialize + Send>),
    FormUrlEncoded(Box<dyn masking::ErasedMaskSerialize + Send>),
    FormData(reqwest::multipart::Form),
    Xml(Box<dyn masking::ErasedMaskSerialize + Send>),
}

impl Request {
        /// Creates a new instance of HttpRequest with the specified HTTP method and URL.
    pub fn new(method: Method, url: &str) -> Self {
        Self {
            method,
            url: String::from(url),
            headers: std::collections::HashSet::new(),
            certificate: None,
            certificate_key: None,
            body: None,
        }
    }

        /// Sets the body of the request to the provided value. The body can be of any type that can be converted into a RequestContent.
    pub fn set_body<T: Into<RequestContent>>(&mut self, body: T) {
        self.body.replace(body.into());
    }

        /// Adds default headers to the existing headers in the request.
    pub fn add_default_headers(&mut self) {
        self.headers.extend(default_request_headers());
    }

        /// Adds a new header to the headers map. If the header already exists, its value will be updated.
    ///
    /// # Arguments
    ///
    /// * `header` - A string slice representing the name of the header.
    /// * `value` - A Maskable<String> representing the value of the header, which can be masked for security purposes.
    ///
    pub fn add_header(&mut self, header: &str, value: Maskable<String>) {
        self.headers.insert((String::from(header), value));
    }

        /// Sets the certificate for the current instance.
    ///
    /// # Arguments
    ///
    /// * `certificate` - An optional string representing the certificate to be added.
    ///
    pub fn add_certificate(&mut self, certificate: Option<String>) {
        self.certificate = certificate;
    }

        /// Adds the given certificate key to the instance.
    /// 
    /// # Arguments
    /// 
    /// * `certificate_key` - An optional String containing the certificate key to be added.
    /// 
    pub fn add_certificate_key(&mut self, certificate_key: Option<String>) {
        self.certificate = certificate_key;
    }
}

#[derive(Debug)]
pub struct RequestBuilder {
    pub url: String,
    pub headers: Headers,
    pub method: Method,
    pub certificate: Option<String>,
    pub certificate_key: Option<String>,
    pub body: Option<RequestContent>,
}

impl RequestBuilder {
        /// Creates a new instance of the struct, with default values for method, url, headers, certificate, certificate_key, and body.
    pub fn new() -> Self {
        Self {
            method: Method::Get,
            url: String::with_capacity(1024),
            headers: std::collections::HashSet::new(),
            certificate: None,
            certificate_key: None,
            body: None,
        }
    }

        /// Sets the URL for the HTTP request and returns the modified instance.
    pub fn url(mut self, url: &str) -> Self {
        self.url = url.into();
        self
    }

        /// Sets the method for the request and returns the modified instance.
    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

        /// Extends the headers of the current instance with default request headers and returns the modified instance.
    pub fn attach_default_headers(mut self) -> Self {
        self.headers.extend(default_request_headers());
        self
    }

        /// Inserts a new header and value pair into the HTTP headers of the request and returns a new instance of the HTTP request with the updated headers.
    /// 
    /// # Arguments
    /// 
    /// * `header` - A string slice representing the name of the header to be inserted.
    /// * `value` - A string slice representing the value of the header to be inserted.
    /// 
    pub fn header(mut self, header: &str, value: &str) -> Self {
        self.headers.insert((header.into(), value.into()));
        self
    }

        /// Adds the provided headers to the existing headers of the HTTP request and returns the modified HTTP request.
    pub fn headers(mut self, headers: Vec<(String, Maskable<String>)>) -> Self {
        self.headers.extend(headers);
        self
    }

        /// Sets the body of the request with the given content and returns the modified request.
    pub fn set_body<T: Into<RequestContent>>(mut self, body: T) -> Self {
        self.body.replace(body.into());
        self
    }

        /// Adds a certificate to the current instance and returns the modified instance.
    /// 
    /// # Arguments
    /// 
    /// - `certificate`: An optional String containing the certificate to be added.
    /// 
    /// # Returns
    /// 
    /// The modified instance with the certificate added.
    pub fn add_certificate(mut self, certificate: Option<String>) -> Self {
        self.certificate = certificate;
        self
    }

        /// Adds a certificate key to the current instance of the struct. 
    /// 
    /// If the `certificate_key` is provided, it will be assigned to the `certificate_key` field of the struct. 
    /// 
    /// # Arguments
    /// 
    /// * `certificate_key` - An optional String containing the certificate key to be added to the struct.
    /// 
    /// # Returns
    /// 
    /// A new instance of the struct with the updated `certificate_key` field.
    pub fn add_certificate_key(mut self, certificate_key: Option<String>) -> Self {
        self.certificate_key = certificate_key;
        self
    }

        /// Builds a Request object using the values stored in the current Builder instance.
    pub fn build(self) -> Request {
        Request {
            method: self.method,
            url: self.url,
            headers: self.headers,
            certificate: self.certificate,
            certificate_key: self.certificate_key,
            body: self.body,
        }
    }
}

impl Default for RequestBuilder {
        /// Returns a new instance of the current type using the `new` method.
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct RequestBody(Secret<String>);

impl RequestBody {
        /// Logs the request body and returns the encoded body wrapped in a Result
    pub fn log_and_get_request_body<T, F>(
        body: T,
        encoder: F,
    ) -> errors::CustomResult<Self, errors::ParsingError>
    where
        F: FnOnce(T) -> errors::CustomResult<String, errors::ParsingError>,
        T: std::fmt::Debug,
    {
        #[cfg(feature = "logs")]
        logger::info!(connector_request_body=?body);
        Ok(Self(Secret::new(encoder(body)?)))
    }

        /// This method takes a RequestContent enum and returns the inner value as a Secret<String>.
    pub fn get_inner_value(request_body: RequestContent) -> Secret<String> {
        match request_body {
            RequestContent::Json(i) => serde_json::to_string(&i).unwrap_or_default().into(),
            RequestContent::FormUrlEncoded(i) => {
                serde_urlencoded::to_string(&i).unwrap_or_default().into()
            }
            RequestContent::Xml(i) => quick_xml::se::to_string(&i).unwrap_or_default().into(),
            RequestContent::FormData(_) => String::new().into(),
        }
    }
}
