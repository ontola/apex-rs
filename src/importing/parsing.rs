use crate::hashtuple::{LookupTable, Statement};
use rio_api::model::{Literal, NamedOrBlankNode, Term};
use rio_api::parser::QuadsParser;
use rio_turtle::{NQuadsParser, TurtleError};
use std::collections::HashMap;

const EMPTY: &str = "";
const BLANK_NODE_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#blankNode";
const NAMED_NODE_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#namedNode";
const STRING_IRI: &str = "http://www.w3.org/2001/XMLSchema#string";
const LANG_STRING_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/**
 * Parse an n-quads formatted string into a map of resources and hextuples.
 */
pub(crate) fn parse(
    lookup_table: &mut LookupTable,
    payload: &[u8],
) -> HashMap<String, Vec<Statement>> {
    let mut docs: HashMap<String, Vec<Statement>> = HashMap::new();

    NQuadsParser::new(payload)
        .unwrap()
        .parse_all(&mut |q| {
            let subj = str_from_iri_or_bn(&q.subject);
            let pred = String::from(q.predicate.iri);
            let graph = str_from_iri_or_bn(&q.graph_name.unwrap());

            create_hashtuple(
                lookup_table,
                &mut docs,
                subj,
                pred,
                str_from_term(q.object),
                graph,
            );
            Ok(()) as Result<(), TurtleError>
        })
        .unwrap();

    docs
}

fn create_hashtuple(
    lookup_table: &mut LookupTable,
    map: &mut HashMap<String, Vec<Statement>>,
    subj: String,
    pred: String,
    obj: [String; 3],
    graph: String,
) {
    let test: Vec<&str> = graph.split("?graph=").collect();
    let delta_op = test.first().unwrap();
    if test.len() < 2 {
        panic!("Quad doesn't contain graph");
    }
    let id = test
        .last()
        .unwrap()
        .split('/')
        .last()
        .expect("Graph not properly formatted");

    map.entry(String::from(id)).or_insert_with(|| vec![]);

    map.get_mut(id).unwrap().push(Statement::new(
        lookup_table.ensure_value(&subj),
        lookup_table.ensure_value(&pred),
        lookup_table.ensure_value(&obj[0]),
        lookup_table.ensure_value(&obj[1]),
        lookup_table.ensure_value(&obj[2]),
        lookup_table.ensure_value(&delta_op.to_string()),
    ));
}

fn str_from_iri_or_bn(t: &NamedOrBlankNode) -> String {
    match t {
        NamedOrBlankNode::BlankNode(bn) => String::from(bn.id),
        NamedOrBlankNode::NamedNode(nn) => String::from(nn.iri),
    }
}

fn str_from_term(t: Term) -> [String; 3] {
    match t {
        Term::BlankNode(bn) => [
            String::from(bn.id),
            String::from(BLANK_NODE_IRI),
            EMPTY.into(),
        ],
        Term::NamedNode(nn) => [
            String::from(nn.iri),
            String::from(NAMED_NODE_IRI),
            EMPTY.into(),
        ],
        Term::Literal(Literal::Simple { value }) => {
            [value.into(), String::from(STRING_IRI), EMPTY.into()]
        }
        Term::Literal(Literal::LanguageTaggedString { value, language }) => {
            [value.into(), String::from(LANG_STRING_IRI), language.into()]
        }
        Term::Literal(Literal::Typed { value, datatype }) => {
            [value.into(), datatype.iri.into(), EMPTY.into()]
        }
    }
}
