use serde;
#[derive(Clone, Debug)]
pub struct AccessTokenAuthentication;

#[derive(Clone, Debug, serde::Serialize)]
pub struct AccessTokenAuth;
