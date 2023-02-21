use axum::extract::Query;
use axum_macros::FromRequest;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use std::{fmt, str::FromStr};

#[derive(FromRequest, Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize)]
#[from_request(via(Query))]
pub enum Provider {
    #[serde(rename = "all")]
    All,
    #[serde(rename = "phone")]
    Phone,
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "google")]
    Google,
    #[serde(rename = "microsoft")]
    Microsoft,
    #[serde(rename = "linkedin")]
    LinkedIn,
    #[serde(rename = "shopify")]
    Shopify,
    #[serde(rename = "hubspot")]
    Hubspot,
    #[serde(rename = "salesforce")]
    SalesForce,
    #[serde(rename = "zapier")]
    Zapier,
    #[serde(rename = "klaviyo")]
    Klaviyo,
    #[serde(rename = "")]
    DefaultProvider,
}

impl Default for Provider {
    fn default() -> Self {
        Self::DefaultProvider
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for Provider {
    type Err = ();
    fn from_str(input: &str) -> Result<Provider, Self::Err> {
        match input {
            "All" => Ok(Provider::All),
            "Phone"  => Ok(Provider::Phone),
            "Email" => Ok(Provider::Email),
            "Google" => Ok(Provider::Google),
            "Microsoft" => Ok(Provider::Microsoft),
            "LinkedIn" => Ok(Provider::LinkedIn),
            "Shopify" => Ok(Provider::Shopify),
            "Hubspot" => Ok(Provider::Hubspot),
            "Klaviyo" => Ok(Provider::Klaviyo),
            "SalesForce" => Ok(Provider::SalesForce),
            "Zapier" => Ok(Provider::Zapier),
            _       => Err(()),
        }
    }
}