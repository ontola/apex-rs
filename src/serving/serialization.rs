use crate::hashtuple::{HashModel, LookupTable, BLANK_NODE_IRI, LANG_STRING_IRI, NAMED_NODE_IRI};
use rio_api::formatter::{QuadsFormatter, TriplesFormatter};
use rio_api::model::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Quad, Term, Triple};
use rio_turtle::{NQuadsFormatter, NTriplesFormatter, TurtleFormatter};

pub(crate) type Hextuple<'a> = [&'a String; 6];
pub(crate) type HexModel<'a> = Vec<Hextuple<'a>>;
pub(crate) type BulkInput = (Vec<Option<HashModel>>, LookupTable);

pub(crate) const ND_DELIMITER: u8 = b'\n';

pub(crate) fn hash_model_to_hextuples(model: (HashModel, &LookupTable)) -> Vec<u8> {
    let (doc, filled_table) = model;

    let mut output = Vec::new();

    let test = hash_to_hex(doc, &filled_table);
    for h in test {
        output.append(serde_json::to_vec(&h).unwrap().as_mut());
        output.push(ND_DELIMITER);
    }

    output
}

pub(crate) fn hash_model_to_ntriples(model: (HashModel, &LookupTable)) -> Vec<u8> {
    let mut formatter = NTriplesFormatter::new(Vec::default());

    format_model(&mut formatter, model);

    formatter.finish()
}

pub(crate) fn hash_model_to_turtle(model: (HashModel, &LookupTable)) -> Vec<u8> {
    let mut formatter = TurtleFormatter::new(Vec::default());

    format_model(&mut formatter, model);

    formatter.finish().unwrap()
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

pub(crate) fn bulk_result_to_ntriples((docs, filled_table): BulkInput) -> Vec<u8> {
    let mut formatter = NTriplesFormatter::new(Vec::default());

    for doc in docs {
        match doc {
            None => (),
            Some(doc) => {
                format_model(&mut formatter, (doc, &filled_table));
            }
        }
    }

    formatter.finish()
}

pub(crate) fn bulk_result_to_nquads((docs, filled_table): BulkInput) -> Vec<u8> {
    let mut formatter = NQuadsFormatter::new(Vec::default());

    for doc in docs {
        match doc {
            None => (),
            Some(doc) => {
                hash_to_rio(doc, &filled_table).iter().for_each(|term| {
                    formatter.format(&term).unwrap();
                });
            }
        }
    }

    formatter.finish()
}

fn format_model<T>(formatter: &mut T, (doc, filled_table): (HashModel, &LookupTable))
where
    T: TriplesFormatter,
{
    hash_to_rio(doc, &filled_table).iter().for_each(|term| {
        formatter
            .format(&Triple {
                subject: term.subject,
                predicate: term.predicate,
                object: term.object,
            })
            .unwrap()
    });
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

fn hash_to_rio(hashtuples: HashModel, lookup_table: &LookupTable) -> Vec<Quad> {
    let mut vec = Vec::with_capacity(hashtuples.len());

    for h in hashtuples {
        let graph = lookup_table.get_by_hash(h.graph);
        vec.push(Quad {
            subject: to_named_or_blanknode(lookup_table.get_by_hash(h.subject)),
            predicate: NamedNode {
                iri: lookup_table.get_by_hash(h.predicate),
            },
            object: to_object(
                lookup_table.get_by_hash(h.value),
                lookup_table.get_by_hash(h.datatype),
                lookup_table.get_by_hash(h.language),
            ),
            graph_name: if graph == "" {
                None
            } else {
                Some(to_named_or_blanknode(graph))
            },
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
