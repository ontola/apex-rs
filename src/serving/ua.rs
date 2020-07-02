pub(crate) static APEX_UA_BASE: &str = "Apex";
pub(crate) static APEX_UA_VERSION: &str = "1";
pub(crate) static APEX_UA_BULK_REQ: &str = "bulk proxy";

#[inline]
pub fn basic_ua() -> String {
    format!("{}/{}", APEX_UA_BASE, APEX_UA_VERSION)
}

#[inline]
pub fn comment_ua(comment: &str) -> String {
    format!("{} ({})", basic_ua(), comment)
}

#[inline]
pub fn bulk_ua() -> String {
    comment_ua(APEX_UA_BULK_REQ)
}
