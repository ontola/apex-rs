use bimap::{BiMap, Overwritten};
use fasthash::murmur3;

pub(crate) const BLANK_NODE_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#blankNode";
pub(crate) const NAMED_NODE_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#namedNode";
pub(crate) const SUBJECT_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#subject";
pub(crate) const PREDICATE_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate";
pub(crate) const OBJECT_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#object";
pub(crate) const STRING_IRI: &str = "http://www.w3.org/2001/XMLSchema#string";
pub(crate) const LANG_STRING_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// An RDF statement composed of six elements, each element contains the murmur3 hash
/// of the values string representation.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct Statement {
    pub subject: u128,
    pub predicate: u128,
    pub value: u128,
    pub datatype: u128,
    pub language: u128,
    pub graph: u128, // FIXME: PartialOrd, Ord, should this be top or bottom?
}

pub type HashModel = Vec<Statement>;

impl Statement {
    #[inline(always)]
    pub fn new(
        subject: u128,
        predicate: u128,
        value: u128,
        datatype: u128,
        language: u128,
        graph: u128,
    ) -> Statement {
        Statement {
            subject,
            predicate,
            value,
            datatype,
            language,
            graph,
        }
    }
}

/// Mapping between hextuple string values and their hashed ids.
#[derive(Debug)]
pub struct LookupTable {
    map: BiMap<u128, String>,
    seed: u32,
}

impl LookupTable {
    pub fn new(seed: u32) -> LookupTable {
        LookupTable {
            map: BiMap::new(),
            seed,
        }
    }

    pub fn ensure_value(&mut self, value: &str) -> u128 {
        let id = self.calculate_hash(&value);
        let update = self.map.insert(id, value.to_string());
        match update {
            Overwritten::Right(_, _) | Overwritten::Left(_, _) => panic!("Hash collision detected"),
            _ => (),
        }

        id
    }

    pub fn calculate_hash(&self, value: &str) -> u128 {
        murmur3::hash128_with_seed(&value, self.seed)
    }

    pub fn get_by_hash(&self, id: u128) -> Option<&String> {
        self.map.get_by_left(&id)
    }

    pub fn get_by_value(&self, value: String) -> u128 {
        *self.map.get_by_right(&value).unwrap()
    }
}
