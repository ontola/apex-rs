use bimap::{BiMap, Overwritten};
use fasthash::murmur3;

/// An RDF statement formatted into a tuple of six elements, each element contains the murmur3 hash
/// of the values string representation.
///
/// The order is as follows:
/// [subject, predicate, object value, object, datatype, object language, graph]
pub type Hashtuple = [u128; 6];

pub type HashModel = Vec<Hashtuple>;

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
