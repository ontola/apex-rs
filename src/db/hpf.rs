//! Hex Pattern Fragments database implementation
//!
//! HexPF differs from TripleFP in that the object is decomposed into its three components;
//! value, data type, and language which can all be queried independently resulting in greater query
//! capacity.
//!
//! For example, one could request to match all values matching "1.5" independently of
//! data type, or request all named nodes from an object skipping literals.

use crate::db::db_context::{verified_ensure, DbContext};
use crate::db::models::{Document, Object, Property, Resource};
use crate::db::properties::MAX_PROPERTY_INSERT_SIZE;
use crate::db::schema::documents::dsl as documents;
use crate::db::schema::objects::dsl as objects;
use crate::db::schema::properties;
use crate::db::schema::resources::dsl as resources;
use crate::db::uu128::Uu128;
use crate::errors::ErrorKind;
use crate::hashtuple::{
    HashModel, Statement, NAMED_NODE_IRI, OBJECT_IRI, PREDICATE_IRI, STRING_IRI, SUBJECT_IRI,
};
use actix_web::Either;
use diesel::prelude::*;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Deserialize)]
pub(crate) struct HPFQueryRequest {
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    predicate: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    datatype: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    page: Option<i64>,
    #[serde(default)]
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TPFQueryRequest {
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    predicate: Option<String>,
    #[serde(default)]
    object: Option<String>,
    #[serde(default)]
    page: Option<i64>,
    #[serde(default)]
    page_size: Option<i64>,
}

pub(crate) struct Variable {
    /// Unclear if and how this should be conveyed in the response
    _name: String,
}

pub(crate) type VarOrIRI = Either<Variable, String>;
pub(crate) type VarOrId = Either<Variable, i32>;
pub(crate) type VarOrHash = Either<Variable, Uu128>;

pub(crate) struct HPFQuery {
    page_size: i64,
    /// The property id to start from
    from: i64,

    pub subject: VarOrIRI,
    pub predicate: VarOrId,
    pub value: VarOrHash,
    pub datatype: VarOrId,
    pub language: VarOrId,
}

impl HPFQuery {
    pub fn parse(
        mut db_ctx: &mut DbContext,
        request: &HPFQueryRequest,
    ) -> Result<HPFQuery, ErrorKind> {
        let subject = parse_subject(&request.subject);
        let predicate = parse_predicate(&mut db_ctx, &request.predicate)?;
        let value = parse_value(&mut db_ctx, &request.value)?;
        let datatype = parse_datatype(&mut db_ctx, &request.datatype)?;
        let language = parse_language(&mut db_ctx, &request.language)?;

        Ok(HPFQuery {
            page_size: request.page_size.unwrap_or(500).abs().min(100_000).max(1),
            from: request.page.unwrap_or(0).max(0),

            subject,
            predicate,
            value,
            datatype,
            language,
        })
    }

    pub fn parse_tpf(
        mut db_ctx: &mut DbContext,
        request: &TPFQueryRequest,
    ) -> Result<HPFQuery, ErrorKind> {
        let subject = parse_subject(&request.subject);
        let predicate = parse_predicate(&mut db_ctx, &request.predicate)?;
        let (value, datatype, language) = parse_object(&mut db_ctx, &request.object)?;

        Ok(HPFQuery {
            page_size: request.page_size.unwrap_or(500).abs().min(100_000).max(1),
            from: request.page.unwrap_or(0).max(0),

            subject,
            predicate,
            value,
            datatype,
            language,
        })
    }

    pub fn header(&self, mut db_ctx: &mut DbContext, origin: &str) -> Result<HashModel, ErrorKind> {
        let dataset = db_ctx
            .lookup_table
            .ensure_value(&format!("{}#dataset", origin));
        let template_iri = db_ctx
            .lookup_table
            .ensure_value(&format!("{}/tpf#template", origin));
        let named_node = db_ctx.lookup_table.ensure_value(NAMED_NODE_IRI);
        let empty = db_ctx.lookup_table.ensure_value("");

        let dataset = vec![
            Statement::new(
                dataset,
                db_ctx
                    .lookup_table
                    .ensure_value("http://rdfs.org/ns/void#subset"),
                db_ctx
                    .lookup_table
                    .ensure_value(&self.collection_iri(&db_ctx, origin)),
                named_node,
                empty,
                empty,
            ),
            Statement::new(
                dataset,
                db_ctx
                    .lookup_table
                    .ensure_value("http://www.w3.org/ns/hydra/core#search"),
                template_iri,
                named_node,
                empty,
                empty,
            ),
        ];

        let model = [
            dataset.as_slice(),
            template_statements(&mut db_ctx, origin).as_slice(),
        ]
        .concat();

        Ok(model)
    }

    pub fn collection_iri(&self, db_ctx: &DbContext, origin: &str) -> String {
        let base = format!("{}/tpf", origin);
        let mut map = HashMap::new();

        if let Either::B(iri) = &self.subject {
            map.insert("subject", iri.to_string());
        }
        if let Either::B(predicate_id) = self.predicate {
            map.insert(
                "predicate",
                db_ctx
                    .property_map
                    .get_by_right(&predicate_id)
                    .unwrap()
                    .to_string(),
            );
        }
        if let Either::B(object_id) = self.value {
            let t = db_ctx.lookup_table.get_by_hash(u128::from(object_id));
            if t.is_none() {
                panic!("Value not in map")
            }
            map.insert("object", t.unwrap().to_string());
        }

        if map.len() > 0 {
            format!("{}?{}", base, serde_qs::to_string(&map).unwrap())
        } else {
            base
        }
    }

    pub fn execute(&self, mut db_ctx: &mut DbContext) -> Result<HashModel, ErrorKind> {
        use properties::dsl;

        let conn = db_ctx.get_conn();
        debug!(target: "apex", "TPF: Retrieving max {} triples from id {}", self.page_size, self.from);
        let mut q = dsl::properties
            .filter(dsl::id.gt(self.from))
            .limit(self.page_size)
            .into_boxed();

        match &self.subject {
            Either::A(_var) => (),
            Either::B(val) => {
                let t = resources::resources
                    .inner_join(documents::documents)
                    .filter(documents::iri.eq(val))
                    .load::<(Resource, Document)>(&conn)
                    .unwrap();

                let mut resource_ids = HashSet::new();

                t.iter().for_each(|(resource, doc)| {
                    db_ctx.lookup_table.ensure_value(&doc.iri);
                    resource_ids.insert(resource.id);
                });

                q = q.filter(dsl::resource_id.eq_any(resource_ids))
            }
        };

        match &self.predicate {
            Either::A(_var) => (),
            Either::B(val) => {
                q = q.filter(dsl::predicate_id.eq(val));
            }
        };

        match &self.value {
            Either::A(_var) => (),
            Either::B(val) => {
                q = q.filter(dsl::object_id.eq(val));
            }
        };

        match &self.datatype {
            Either::A(_var) => (),
            Either::B(val) => {
                q = q.filter(dsl::datatype_id.eq(val));
            }
        };

        match &self.language {
            Either::A(_var) => (),
            Either::B(val) => {
                q = q.filter(dsl::language_id.eq(val));
            }
        };

        let matches = q.load::<Property>(&conn).unwrap();

        ensure_subjects(&mut db_ctx, &matches);
        ensure_objects(&mut db_ctx, &matches);

        let statements = matches
            .iter()
            .map(|p: &Property| Statement {
                subject: db_ctx
                    .lookup_table
                    .calculate_hash(db_ctx.resource_map.get_by_right(&p.resource_id).unwrap()),
                predicate: db_ctx
                    .lookup_table
                    .ensure_value(db_ctx.property_map.get_by_right(&p.predicate_id).unwrap()),
                value: u128::from(p.object_id.unwrap()),
                datatype: db_ctx.lookup_table.ensure_value(
                    db_ctx
                        .datatype_map
                        .get_by_right(&p.datatype_id)
                        .expect("Datatype id not in map"),
                ),
                language: db_ctx
                    .lookup_table
                    .ensure_value(db_ctx.property_map.get_by_right(&p.predicate_id).unwrap()),
                graph: db_ctx.lookup_table.ensure_value(""),
            })
            .collect();

        Ok(statements)
    }
}

fn ensure_subjects(db_ctx: &mut DbContext, matches: &Vec<Property>) {
    let mut subject_ids = HashSet::new();

    matches.iter().for_each(|p| {
        subject_ids.insert(p.resource_id);
    });

    debug!(target: "apex", "Ensuring {} subjects", subject_ids.len());
    subject_ids
        .into_iter()
        .collect::<Vec<_>>()
        .chunks(MAX_PROPERTY_INSERT_SIZE)
        .for_each(|chunk| {
            let found_subjects = resources::resources
                .filter(resources::id.eq_any(chunk))
                .get_results::<Resource>(&db_ctx.get_conn())
                .unwrap();

            for o in found_subjects {
                db_ctx.lookup_table.ensure_value(&o.iri);
                verified_ensure(
                    &mut db_ctx.resource_map,
                    o.iri,
                    o.id,
                    "Subject or hash already present",
                );
            }
        });
}

fn ensure_objects(db_ctx: &mut DbContext, matches: &Vec<Property>) {
    let mut object_ids = HashSet::new();

    matches.iter().for_each(|p| {
        object_ids.insert(p.object_id.unwrap());
    });

    debug!(target: "apex", "Ensuring {} objects", object_ids.len());
    object_ids
        .into_iter()
        .collect::<Vec<_>>()
        .chunks(MAX_PROPERTY_INSERT_SIZE)
        .for_each(|chunk| {
            let found_objects = objects::objects
                .filter(objects::hash.eq_any(chunk))
                .get_results::<Object>(&db_ctx.get_conn())
                .unwrap();

            for o in found_objects {
                db_ctx.lookup_table.ensure_value(&o.value);
            }
        });
}

fn parse_subject(s: &Option<String>) -> VarOrIRI {
    match s {
        None => Either::A(Variable {
            _name: "anonymous".into(),
        }),
        Some(s) => match s.get(..1) {
            None | Some("") => Either::A(Variable {
                _name: "anonymous".into(),
            }),
            Some("?") => Either::A(Variable {
                _name: String::from(&s[1..]),
            }),
            Some(_) => Either::B(s.into()),
        },
    }
}

fn parse_predicate(db_ctx: &mut DbContext, s: &Option<String>) -> Result<VarOrId, ErrorKind> {
    match s {
        None => Ok(Either::A(Variable {
            _name: "anonymous".into(),
        })),
        Some(s) => match s.get(..1) {
            None | Some("") => Ok(Either::A(Variable {
                _name: "anonymous".into(),
            })),
            Some("?") => Ok(Either::A(Variable {
                _name: String::from(&s[1..]),
            })),
            Some(_) => match db_ctx.property_map.get_by_left(s) {
                Some(id) => Ok(Either::B(*id)),
                None => Err(ErrorKind::NoResources),
            },
        },
    }
}

fn parse_value(db_ctx: &mut DbContext, s: &Option<String>) -> Result<VarOrHash, ErrorKind> {
    if s.is_none() || s.as_ref().unwrap() == "" {
        return Ok(Either::A(Variable {
            _name: "anonymous".into(),
        }));
    }

    let value = Uu128::from(db_ctx.lookup_table.ensure_value(&s.as_ref().unwrap()));

    Ok(Either::B(value))
}

fn parse_datatype(db_ctx: &DbContext, s: &Option<String>) -> Result<VarOrId, ErrorKind> {
    if s.is_none() || s.as_ref().unwrap() == "" {
        return Ok(Either::A(Variable {
            _name: "anonymous".into(),
        }));
    }

    let datatype_id = *db_ctx
        .datatype_map
        .get_by_left(s.as_ref().unwrap())
        .unwrap();

    Ok(Either::B(datatype_id))
}

fn parse_language(db_ctx: &DbContext, s: &Option<String>) -> Result<VarOrId, ErrorKind> {
    if s.is_none() || s.as_ref().unwrap() == "" {
        return Ok(Either::A(Variable {
            _name: "anonymous".into(),
        }));
    }

    let language_id = *db_ctx
        .language_map
        .get_by_left(s.as_ref().unwrap())
        .unwrap();

    Ok(Either::B(language_id))
}

fn parse_object(
    db_ctx: &mut DbContext,
    s: &Option<String>,
) -> Result<(VarOrHash, VarOrId, VarOrId), ErrorKind> {
    if s.is_none() || s.as_ref().unwrap() == "" {
        return Ok((
            Either::A(Variable {
                _name: "anonymous".into(),
            }),
            Either::A(Variable {
                _name: "anonymous".into(),
            }),
            Either::A(Variable {
                _name: "anonymous".into(),
            }),
        ));
    }

    let s = s.as_ref().unwrap();
    if s.len() < 2 {
        return Err(ErrorKind::ParserError(String::from(
            "Invalid object parameter format",
        )));
    }
    if s.get(..1).unwrap() == "?" {
        let name = &s[1..];

        return Ok((
            Either::A(Variable {
                _name: String::from(name),
            }),
            Either::A(Variable {
                _name: String::from(name),
            }),
            Either::A(Variable {
                _name: String::from(name),
            }),
        ));
    }

    if s.starts_with('"') {
        let second = s.chars().skip(1).position(|c| c == '"').unwrap();
        let t = db_ctx.lookup_table.ensure_value(&s[1..second + 1]);
        let value = Uu128::from(t);

        let (datatype_id, lang_id) = if s.contains("^^") {
            let val_datatype = match s.split("^^").last() {
                Some(s) => Ok(s),
                None => Err(ErrorKind::ParserError(
                    "Invalid object parameter formatting".into(),
                )),
            }?;
            let datatype_id = *db_ctx
                .datatype_map
                .get_by_left(&val_datatype.into())
                .unwrap();
            let language_id = *db_ctx.language_map.get_by_left(&"".into()).unwrap();

            (datatype_id, language_id)
        } else if s.contains("@") {
            let val_datatype = s.split("@").collect::<Vec<&str>>();
            let datatype_id = *db_ctx
                .datatype_map
                .get_by_left(&val_datatype[1].into())
                .unwrap();
            let language_id = *db_ctx.language_map.get_by_left(&"".into()).unwrap();

            (datatype_id, language_id)
        } else {
            let datatype_id = *db_ctx.datatype_map.get_by_left(&STRING_IRI.into()).unwrap();
            // let language_id = *db_ctx.language_map.get("").unwrap();

            (datatype_id, 0)
        };

        Ok((Either::B(value), Either::B(datatype_id), Either::B(lang_id)))
    } else {
        let value = Uu128::from(db_ctx.lookup_table.ensure_value(&s));

        Ok((
            Either::B(value),
            Either::B(
                *db_ctx
                    .datatype_map
                    .get_by_left(&NAMED_NODE_IRI.into())
                    .unwrap(),
            ),
            Either::B(*db_ctx.language_map.get_by_left(&"".into()).unwrap()),
        ))
    }
}

fn template_statements(db_ctx: &mut DbContext, origin: &str) -> Vec<Statement> {
    let named_node = db_ctx.lookup_table.get_by_value(NAMED_NODE_IRI.into());
    let string_type = db_ctx.lookup_table.ensure_value(STRING_IRI);
    let hydra_mapping = db_ctx
        .lookup_table
        .ensure_value("http://www.w3.org/ns/hydra/core#mapping");
    let hydra_property = db_ctx
        .lookup_table
        .ensure_value("http://www.w3.org/ns/hydra/core#property");
    let hydra_template = db_ctx
        .lookup_table
        .ensure_value("http://www.w3.org/ns/hydra/core#template");
    let hydra_variable = db_ctx
        .lookup_table
        .ensure_value("http://www.w3.org/ns/hydra/core#variable");

    let empty = db_ctx.lookup_table.get_by_value("".into());
    let tmpl_base_iri = format!("{}/tpf#template", origin);
    let template_iri = db_ctx.lookup_table.ensure_value(&tmpl_base_iri);
    let template_subject_iri = db_ctx
        .lookup_table
        .ensure_value(&format!("{}_subject", tmpl_base_iri));
    let template_predicate_iri = db_ctx
        .lookup_table
        .ensure_value(&format!("{}_predicate", tmpl_base_iri));
    let template_object_iri = db_ctx
        .lookup_table
        .ensure_value(&format!("{}_object", tmpl_base_iri));

    vec![
        Statement::new(
            template_iri,
            hydra_template,
            db_ctx
                .lookup_table
                .ensure_value(&format!("{}/tpf{{?subject,?predicate,?object}}", origin)),
            string_type,
            empty,
            empty,
        ),
        Statement::new(
            template_iri,
            hydra_mapping,
            template_subject_iri,
            named_node,
            empty,
            empty,
        ),
        Statement::new(
            template_iri,
            hydra_mapping,
            template_predicate_iri,
            named_node,
            empty,
            empty,
        ),
        Statement::new(
            template_iri,
            hydra_mapping,
            template_object_iri,
            named_node,
            empty,
            empty,
        ),
        Statement::new(
            template_subject_iri,
            hydra_variable,
            db_ctx.lookup_table.ensure_value("subject".into()),
            string_type,
            empty,
            empty,
        ),
        Statement::new(
            template_subject_iri,
            hydra_property,
            db_ctx.lookup_table.ensure_value(&SUBJECT_IRI),
            named_node,
            empty,
            empty,
        ),
        Statement::new(
            template_predicate_iri,
            hydra_variable,
            db_ctx.lookup_table.ensure_value("predicate".into()),
            string_type,
            empty,
            empty,
        ),
        Statement::new(
            template_predicate_iri,
            hydra_property,
            db_ctx.lookup_table.ensure_value(&PREDICATE_IRI),
            named_node,
            empty,
            empty,
        ),
        Statement::new(
            template_object_iri,
            hydra_variable,
            db_ctx.lookup_table.ensure_value("object".into()),
            string_type,
            empty,
            empty,
        ),
        Statement::new(
            template_object_iri,
            hydra_property,
            db_ctx.lookup_table.ensure_value(&OBJECT_IRI),
            named_node,
            empty,
            empty,
        ),
    ]
}
