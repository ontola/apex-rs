use crate::app_config::ClusterConfig;
use crate::errors::ErrorKind;
use url::Url;

pub fn route(config: &ClusterConfig, iri: &str) -> Result<Option<String>, ErrorKind> {
    let mut url =
        Url::parse(&iri).map_err(|_| ErrorKind::ParserError("Couldn't parse iri".into()))?;
    let path = String::from(url.path());

    if let Some(port) = config.default_service_port.clone() {
        url.set_port(Some(port)).unwrap();
    }

    let deku_match = regex::Regex::new(r"^/\w*/\w*/od/?.*$").unwrap();
    let email_match = regex::Regex::new(r"^/email/").unwrap();
    let subscribe_match = regex::Regex::new(r"^/subscribe").unwrap();
    let token_match = regex::Regex::new(r"^(/\w+)?/tokens").unwrap();
    let vote_compare_match = regex::Regex::new(r"^/compare/votes").unwrap();

    let service_name = if deku_match.is_match(&path) {
        "deku"
    } else if email_match.is_match(&path) {
        "email"
    } else if subscribe_match.is_match(&path) {
        "subscribe"
    } else if token_match.is_match(&path) {
        "token"
    } else if vote_compare_match.is_match(&path) {
        "vote_compare"
    } else {
        return Ok(None);
    };

    url.set_scheme(&config.default_service_proto)
        .map_err(|_| ErrorKind::Msg("Unexpected error setting scheme".into()))?;
    let host = format!("{}{}", service_name, config.cluster_url_base);
    url.set_host(Some(&host))
        .map_err(|_| ErrorKind::Msg("Unexpected error setting host".into()))?;

    Ok(Some(url.to_string()))
}
