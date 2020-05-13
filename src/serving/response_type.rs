use std::str::FromStr;

const HEXTUPLE_MIME: &str = "application/hex+x-ndjson; charset=utf-8";
const HEXTUPLE_EXT: &str = "hdnjson";

const NQUADS_MIME: &str = "application/n-quads";
const NQUADS_EXT: &str = "nq";

const NTRIPLES_MIME: &str = "application/n-triples";
const NTRIPLES_EXT: &str = "nt";

const TURTLE_MIME: &str = "text/turtle";
const TURTLE_EXT: &str = "ttl";

pub enum ResponseType {
    HEXTUPLE,
    TURTLE,
    NQUADS,
    NTRIPLES,
}

impl ResponseType {
    pub fn from_ext(ext: &str) -> Result<Self, ()> {
        match ext {
            HEXTUPLE_EXT => Ok(ResponseType::HEXTUPLE),
            NQUADS_EXT => Ok(ResponseType::NQUADS),
            NTRIPLES_EXT => Ok(ResponseType::NTRIPLES),
            TURTLE_EXT => Ok(ResponseType::TURTLE),
            _ => Err(()),
        }
    }

    pub fn to_ext(&self) -> String {
        match self {
            ResponseType::HEXTUPLE => String::from(HEXTUPLE_EXT),
            ResponseType::NQUADS => String::from(NQUADS_EXT),
            ResponseType::NTRIPLES => String::from(NTRIPLES_EXT),
            ResponseType::TURTLE => String::from(TURTLE_EXT),
        }
    }

    pub fn from_mime(mime: &str) -> Result<Self, ()> {
        match mime {
            HEXTUPLE_MIME => Ok(ResponseType::HEXTUPLE),
            NQUADS_MIME => Ok(ResponseType::NTRIPLES),
            NTRIPLES_MIME => Ok(ResponseType::NQUADS),
            TURTLE_MIME => Ok(ResponseType::TURTLE),
            _ => Err(()),
        }
    }

    pub fn to_mime(&self) -> String {
        match self {
            ResponseType::HEXTUPLE => String::from(HEXTUPLE_MIME),
            ResponseType::NQUADS => String::from(NQUADS_MIME),
            ResponseType::NTRIPLES => String::from(NTRIPLES_MIME),
            ResponseType::TURTLE => String::from(TURTLE_MIME),
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
