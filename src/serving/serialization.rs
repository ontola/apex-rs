use crate::hashtuple::{HashModel, LookupTable, BLANK_NODE_IRI, LANG_STRING_IRI, NAMED_NODE_IRI};
use rio_api::formatter::TriplesFormatter;
use rio_api::model::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Term, Triple};
use rio_turtle::{NTriplesFormatter, TurtleFormatter};

pub(crate) type Hextuple<'a> = [&'a String; 6];
pub(crate) type HexModel<'a> = Vec<Hextuple<'a>>;
pub(crate) type BulkInput = (Vec<Option<HashModel>>, LookupTable);

pub(crate) const ND_DELIMITER: u8 = b'\n';

pub(crate) fn hash_model_to_hextuples(model: (HashModel, LookupTable)) -> Vec<u8> {
    let (doc, filled_table) = model;

    let mut output = Vec::new();

    let test = hash_to_hex(doc, &filled_table);
    for h in test {
        output.append(serde_json::to_vec(&h).unwrap().as_mut());
        output.push(ND_DELIMITER);
    }

    output
}

pub(crate) fn bulk_result_to_hextuples((docs, filled_table): BulkInput) -> Vec<u8> {
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

    output
}

pub(crate) fn hash_model_to_ntriples((doc, filled_table): (HashModel, LookupTable)) -> Vec<u8> {
    let mut formatter = NTriplesFormatter::new(Vec::default());

    hash_to_rio(doc, &filled_table)
        .iter()
        .for_each(|term| formatter.format(&term).unwrap());

    formatter.finish()
}

pub(crate) fn hash_model_to_turtle((doc, filled_table): (HashModel, LookupTable)) -> Vec<u8> {
    let mut formatter = TurtleFormatter::new(Vec::default());

    hash_to_rio(doc, &filled_table)
        .iter()
        .for_each(|term| formatter.format(&term).unwrap());

    formatter.finish().unwrap()
}

fn hash_to_hex(hashtuples: HashModel, lookup_table: &LookupTable) -> HexModel {
    let mut vec = Vec::with_capacity(hashtuples.len());
    for h in hashtuples {
        vec.push([
            lookup_table.get_by_hash(h.subject),
            lookup_table.get_by_hash(h.predicate),
            lookup_table.get_by_hash(h.value),
            lookup_table.get_by_hash(h.datatype),
            lookup_table.get_by_hash(h.language),
            lookup_table.get_by_hash(h.graph),
        ]);
    }

    vec
}

fn hash_to_rio(hashtuples: HashModel, lookup_table: &LookupTable) -> Vec<Triple> {
    let mut vec = Vec::with_capacity(hashtuples.len());
    for h in hashtuples {
        vec.push(Triple {
            subject: to_named_or_blanknode(lookup_table.get_by_hash(h.subject)),
            predicate: NamedNode {
                iri: lookup_table.get_by_hash(h.predicate),
            },
            object: to_object(
                lookup_table.get_by_hash(h.value),
                lookup_table.get_by_hash(h.datatype),
                lookup_table.get_by_hash(h.language),
            ),
        });
    }

    vec
}

fn to_named_or_blanknode(value: &String) -> NamedOrBlankNode {
    if value.contains(':') {
        NamedOrBlankNode::NamedNode(NamedNode { iri: value })
    } else {
        NamedOrBlankNode::BlankNode(BlankNode {
            id: value[2..].into(),
        })
    }
}

fn to_object<'a>(value: &'a String, datatype: &'a String, language: &'a String) -> Term<'a> {
    match datatype.as_str() {
        BLANK_NODE_IRI => Term::BlankNode(BlankNode {
            id: value[2..].into(),
        }),
        NAMED_NODE_IRI => Term::NamedNode(NamedNode { iri: value }),
        LANG_STRING_IRI => Term::Literal(Literal::LanguageTaggedString { value, language }),
        _ => Term::Literal(Literal::Typed {
            value,
            datatype: NamedNode { iri: datatype },
        }),
    }
}
