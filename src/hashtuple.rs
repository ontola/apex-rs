use bimap::{BiMap, Overwritten};
use fasthash::murmur3;

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
pub struct LookupTable(pub BiMap<u128, String>);

impl LookupTable {
    pub fn new() -> LookupTable {
        LookupTable { 0: BiMap::new() }
    }

    pub fn ensure_value(&mut self, value: &String) -> u128 {
        let id = murmur3::hash128(&value);
        let update = self.0.insert(id, value.to_string());
        match update {
            Overwritten::Right(_, _) | Overwritten::Left(_, _) => panic!("Hash collision detected"),
            _ => (),
        }

        id
    }

    pub fn get_by_hash(&self, id: u128) -> &String {
        &self.0.get_by_left(&id).unwrap()
    }

    pub fn get_by_value(&self, value: String) -> u128 {
        *self.0.get_by_right(&value).unwrap()
    }
}
