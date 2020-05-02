use crate::hashtuple::{HashModel, LookupTable};
use actix_web::error::BlockingError;
use actix_web::HttpResponse;

pub(crate) type Hextuple<'a> = [&'a String; 6];
pub(crate) type HexModel<'a> = Vec<Hextuple<'a>>;
pub(crate) type BulkInput = (Vec<Option<HashModel>>, LookupTable);

pub(crate) const ND_DELIMITER: u8 = b'\n';

pub(crate) fn hash_model_to_response(
    model: Result<(HashModel, LookupTable), BlockingError<i32>>,
) -> HttpResponse {
    match model {
        Err(_code) => HttpResponse::NotFound().finish(),
        Ok((doc, filled_table)) => {
            let mut output = Vec::new();

            let test = hash_to_hex(doc, &filled_table);
            for h in test {
                output.append(serde_json::to_vec(&h).unwrap().as_mut());
                output.push(ND_DELIMITER);
            }

            HttpResponse::Ok()
                // .content_type("application/hex+ndjson")
                .body(output)
        }
    }
}

pub(crate) fn bulk_result_to_response(
    model: Result<BulkInput, BlockingError<i32>>,
) -> HttpResponse {
    match model {
        Err(_code) => HttpResponse::NotFound().finish(),
        Ok((docs, filled_table)) => {
            let mut output = Vec::new();

            for doc in docs {
                match doc {
                    None => (),
                    Some(doc) => {
                        let test = hash_to_hex(doc, &filled_table);
                        for h in test {
                            output.append(serde_json::to_vec(&h).unwrap().as_mut());
                            output.push(ND_DELIMITER);
                        }
                    }
                }
            }

            HttpResponse::Ok()
                // .content_type("application/hex+ndjson")
                .body(output)
        }
    }
}

fn hash_to_hex(hashtuples: HashModel, lookup_table: &LookupTable) -> HexModel {
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
