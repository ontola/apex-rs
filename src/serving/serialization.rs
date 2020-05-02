use crate::hashtuple::{HashModel, LookupTable};

pub(crate) type Hextuple<'a> = [&'a String; 6];
pub(crate) type HexModel<'a> = Vec<Hextuple<'a>>;

pub(crate) const ND_DELIMITER: u8 = b'\n';

pub(crate) fn hash_to_hex(hashtuples: HashModel, lookup_table: &LookupTable) -> HexModel {
    let mut vec = Vec::with_capacity(hashtuples.len());
    for h in hashtuples {
        vec.push([
            lookup_table.get_by_hash(h[0]),
            lookup_table.get_by_hash(h[1]),
            lookup_table.get_by_hash(h[2]),
            lookup_table.get_by_hash(h[3]),
            lookup_table.get_by_hash(h[4]),
            lookup_table.get_by_hash(h[5]),
        ]);
    }

    vec
}
