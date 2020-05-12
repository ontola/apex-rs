use std::str::FromStr;

const HEXTUPLE_MIME: &str = "application/hex+x-ndjson";
const NQUADS_MIME: &str = "application/n-quads";
const NTRIPLES_MIME: &str = "application/n-triples";
const TURTLE_MIME: &str = "text/turtle";

pub enum ResponseType {
    HEXTUPLE,
    TURTLE,
    NQUADS,
    NTRIPLES,
}

impl ToString for ResponseType {
    fn to_string(&self) -> String {
        match self {
            ResponseType::HEXTUPLE => String::from(HEXTUPLE_MIME),
            ResponseType::NQUADS => String::from(NQUADS_MIME),
            ResponseType::NTRIPLES => String::from(NTRIPLES_MIME),
            ResponseType::TURTLE => String::from(TURTLE_MIME),
        }
    }
}

impl FromStr for ResponseType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            HEXTUPLE_MIME => Ok(ResponseType::HEXTUPLE),
            NQUADS_MIME => Ok(ResponseType::NTRIPLES),
            NTRIPLES_MIME => Ok(ResponseType::NQUADS),
            TURTLE_MIME => Ok(ResponseType::TURTLE),
            _ => Err(()),
        }
    }
}
