use std::str::FromStr;

pub(crate) const HEXTUPLE_MIME: &str = "application/hex+x-ndjson; charset=utf-8";
pub(crate) const HEXTUPLE_MIME_BASE: &str = "application/hex+x-ndjson";
pub(crate) const HEXTUPLE_EXT: &str = "hdnjson";

pub(crate) const NQUADS_MIME: &str = "application/n-quads";
pub(crate) const NQUADS_EXT: &str = "nq";

pub(crate) const NTRIPLES_MIME: &str = "application/n-triples";
pub(crate) const NTRIPLES_EXT: &str = "nt";

pub(crate) const TURTLE_MIME: &str = "text/turtle";
pub(crate) const TURTLE_EXT: &str = "ttl";

pub(crate) const JSONLD_MIME: &str = "application/ld+json";
pub(crate) const JSONLD_EXT: &str = "jsonld";

pub(crate) const JSON_MIME: &str = "application/json";
pub(crate) const JSON_EXT: &str = "json";

pub enum ResponseType {
    HEXTUPLE,
    TURTLE,
    NQUADS,
    NTRIPLES,
    JSONLD,
    JSON,
}

impl ResponseType {
    pub fn from_ext(ext: &str) -> Result<Self, ()> {
        match ext {
            HEXTUPLE_EXT => Ok(ResponseType::HEXTUPLE),
            NQUADS_EXT => Ok(ResponseType::NQUADS),
            NTRIPLES_EXT => Ok(ResponseType::NTRIPLES),
            TURTLE_EXT => Ok(ResponseType::TURTLE),
            JSONLD_EXT => Ok(ResponseType::JSONLD),
            JSON_EXT => Ok(ResponseType::JSON),
            _ => Err(()),
        }
    }

    pub fn to_ext(&self) -> String {
        match self {
            ResponseType::HEXTUPLE => String::from(HEXTUPLE_EXT),
            ResponseType::NQUADS => String::from(NQUADS_EXT),
            ResponseType::NTRIPLES => String::from(NTRIPLES_EXT),
            ResponseType::TURTLE => String::from(TURTLE_EXT),
            ResponseType::JSONLD => String::from(JSONLD_EXT),
            ResponseType::JSON => String::from(JSON_EXT),
        }
    }

    pub fn from_mime(mime: &str) -> Result<Self, ()> {
        match mime {
            HEXTUPLE_MIME => Ok(ResponseType::HEXTUPLE),
            HEXTUPLE_MIME_BASE => Ok(ResponseType::HEXTUPLE),
            NQUADS_MIME => Ok(ResponseType::NTRIPLES),
            NTRIPLES_MIME => Ok(ResponseType::NQUADS),
            TURTLE_MIME => Ok(ResponseType::TURTLE),
            JSONLD_MIME => Ok(ResponseType::JSONLD),
            JSON_MIME => Ok(ResponseType::JSON),
            _ => Err(()),
        }
    }

    pub fn to_mime(&self) -> String {
        match self {
            ResponseType::HEXTUPLE => String::from(HEXTUPLE_MIME),
            ResponseType::NQUADS => String::from(NQUADS_MIME),
            ResponseType::NTRIPLES => String::from(NTRIPLES_MIME),
            ResponseType::TURTLE => String::from(TURTLE_MIME),
            ResponseType::JSONLD => String::from(JSONLD_MIME),
            ResponseType::JSON => String::from(JSON_MIME),
        }
    }
}

impl ToString for ResponseType {
    fn to_string(&self) -> String {
        self.to_mime()
    }
}

impl FromStr for ResponseType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ResponseType::from_mime(s)
    }
}
