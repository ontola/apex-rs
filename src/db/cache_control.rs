use serde::export::Formatter;
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;

#[derive(PartialEq, Copy, Clone, Debug, Deserialize, Serialize)]
pub(crate) enum CacheControl {
    #[serde(rename = "private")]
    Private,
    #[serde(rename = "no-cache")]
    NoCache,
    #[serde(rename = "public")]
    Public,
}

impl From<i16> for CacheControl {
    fn from(v: i16) -> Self {
        match v {
            0 => CacheControl::Private,
            1 => CacheControl::NoCache,
            2 => CacheControl::Public,
            _ => panic!("Invalid CacheControl range"),
        }
    }
}

impl From<&CacheControl> for String {
    fn from(v: &CacheControl) -> Self {
        match v {
            CacheControl::Private => "private",
            CacheControl::NoCache => "no-cache",
            CacheControl::Public => "public",
        }
        .parse()
        .unwrap()
    }
}

impl Display for CacheControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self))
    }
}
