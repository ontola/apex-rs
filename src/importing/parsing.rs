use crate::errors::ErrorKind;
use crate::hashtuple::{
    LookupTable, Statement, BLANK_NODE_IRI, LANG_STRING_IRI, NAMED_NODE_IRI, STRING_IRI,
};
use percent_encoding::percent_decode_str;
use rio_api::model::{Literal, NamedOrBlankNode, Term};
use rio_api::parser::QuadsParser;
use rio_turtle::{NQuadsParser, TurtleError};
use std::collections::HashMap;
use std::io;
use std::io::{BufRead, BufReader};

/// A set of documents with their IRI as key and their resources as value
pub(crate) type DocumentSet = HashMap<String, Vec<Statement>>;

const EMPTY: &str = "";

pub(crate) fn parse_hndjson<'a>(
    lookup_table: &mut LookupTable,
    payload: &[u8],
) -> Result<DocumentSet, ErrorKind> {
    let mut docs: DocumentSet = HashMap::new();

    let mut data = Vec::new();
    let mut stream = BufReader::new(payload);

    loop {
        let bytes_read = stream.read_until(b'\n', &mut data).unwrap();
        if bytes_read == 0 {
            return Ok(docs);
        }

        let hextuple: Vec<String> = serde_json::from_slice(&data).unwrap();
        data.clear();

        if hextuple.len() != 6 {
            return Err(ErrorKind::ParserError(String::from(
                "Hextuple wasn't 6 long",
            )));
        }

        let subject = hextuple.get(0).unwrap();
        let predicate = hextuple.get(1).unwrap();
        let value = hextuple.get(2).unwrap();
        let datatype = hextuple.get(3).unwrap();
        let language = hextuple.get(4).unwrap();
        let graph = hextuple.get(5).unwrap();

        let res = create_hashtuple(
            lookup_table,
            &mut docs,
            subject,
            predicate,
            value,
            datatype,
            language,
            graph,
        );

        if res.is_err() {
            return Err(res.unwrap_err());
        }
    }
}

/**
 * Parse an n-quads formatted string into a map of resources and hextuples.
 */
pub(crate) fn parse_nquads(
    lookup_table: &mut LookupTable,
    payload: &String,
) -> Result<DocumentSet, ErrorKind> {
    let mut docs: DocumentSet = HashMap::new();

    match NQuadsParser::new(payload.as_bytes()) {
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

                let obj = str_from_term(q.object);
                let res = create_hashtuple(
                    lookup_table,
                    &mut docs,
                    &subj,
                    &pred,
                    &obj[0],
                    &obj[1],
                    &obj[2],
                    &graph,
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

#[allow(clippy::too_many_arguments)]
fn create_hashtuple(
    lookup_table: &mut LookupTable,
    map: &mut DocumentSet,
    subj: &str,
    pred: &str,
    value: &str,
    datatype: &str,
    language: &str,
    graph: &str,
) -> Result<(), ErrorKind> {
    let split_graph: Vec<&str> = graph.split("?graph=").collect();
    let delta_op = match split_graph.first() {
        Some(delta_op) => delta_op,
        None => {
            error!(target: "apex", "Quad doesn't contain graph");
            return Err(ErrorKind::OperatorWithoutGraphName);
        }
    };

    let doc_iri = if split_graph.len() < 2 {
        error!(target: "apex", "Graph is empty, defaulting to subject");

        // return Err(ErrorKind::OperatorWithoutGraphName);
        Some(String::from(subj))
    } else {
        let s = split_graph.last().unwrap();
        let decoded = percent_decode_str(&s).decode_utf8().unwrap();

        Some(decoded.into_owned())
    };

    println!("Hashtuple: {}", delta_op);

    match doc_iri {
        None => {
            error!(target: "apex", "Cannot determine target document");
            Err(ErrorKind::OperatorWithoutGraphName)
        }
        Some(id) => {
            map.entry(String::from(&id)).or_insert_with(|| vec![]);

            map.get_mut(&id).unwrap().push(Statement::new(
                lookup_table.ensure_value(&subj),
                lookup_table.ensure_value(&pred),
                lookup_table.ensure_value(&value),
                lookup_table.ensure_value(&datatype),
                lookup_table.ensure_value(&language),
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
