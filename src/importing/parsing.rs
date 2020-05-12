use crate::errors::ErrorKind;
use crate::hashtuple::{LookupTable, Statement};
use rio_api::model::{Literal, NamedOrBlankNode, Term};
use rio_api::parser::QuadsParser;
use rio_turtle::{NQuadsParser, TurtleError};
use std::collections::HashMap;
use std::io;

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
) -> Result<HashMap<String, Vec<Statement>>, ErrorKind> {
    let mut docs: HashMap<String, Vec<Statement>> = HashMap::new();

    match NQuadsParser::new(payload) {
        Err(e) => Err(ErrorKind::ParserError(e.to_string())),
        Ok(mut model) => {
            // The parse_all method forces TurtleError, so we're circumventing that here.
            let mut real_error = None;
            let result = model.parse_all(&mut |q| -> Result<(), TurtleError> {
                let subj = str_from_iri_or_bn(&q.subject);
                let pred = String::from(q.predicate.iri);
                let graph = match q.graph_name {
                    Some(g) => str_from_iri_or_bn(&g),
                    None => {
                        real_error = Some(ErrorKind::DeltaWithoutOperator);

                        return Err(TurtleError::from(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Unexpected error",
                        )));
                    }
                };

                let res = create_hashtuple(
                    lookup_table,
                    &mut docs,
                    subj,
                    pred,
                    str_from_term(q.object),
                    graph,
                );

                match res {
                    Err(e) => {
                        real_error = Some(e);
                        Err(TurtleError::from(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Unexpected error",
                        )))
                    }
                    Ok(_) => Ok(()),
                }
            });

            match result {
                Ok(_) => Ok(docs),
                Err(_) => Err(real_error.unwrap()),
            }
        }
    }
}

fn create_hashtuple(
    lookup_table: &mut LookupTable,
    map: &mut HashMap<String, Vec<Statement>>,
    subj: String,
    pred: String,
    obj: [String; 3],
    graph: String,
) -> Result<(), ErrorKind> {
    let split_graph: Vec<&str> = graph.split("?graph=").collect();
    let delta_op = match split_graph.first() {
        Some(delta_op) => delta_op,
        None => {
            error!(target: "app", "Quad doesn't contain graph");
            return Err(ErrorKind::OperatorWithoutGraphName);
        }
    };

    if split_graph.len() < 2 {
        error!(target: "app", "Graph is empty");
        return Err(ErrorKind::OperatorWithoutGraphName);
    }

    let last = split_graph.last().unwrap().split('/').last();

    match last {
        None => {
            error!(target: "app", "Operator not properly formatted");
            Err(ErrorKind::DeltaWithoutOperator)
        }
        Some(id) => {
            map.entry(String::from(id)).or_insert_with(|| vec![]);

            map.get_mut(id).unwrap().push(Statement::new(
                lookup_table.ensure_value(&subj),
                lookup_table.ensure_value(&pred),
                lookup_table.ensure_value(&obj[0]),
                lookup_table.ensure_value(&obj[1]),
                lookup_table.ensure_value(&obj[2]),
                lookup_table.ensure_value(&delta_op.to_string()),
            ));

            Ok(())
        }
    }
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
