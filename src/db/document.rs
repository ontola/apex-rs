use crate::db::db_context::DbContext;
use crate::db::models::*;
use crate::db::schema;
use crate::db::schema::objects::dsl as objects;
use crate::errors::ErrorKind;
use crate::hashtuple::{HashModel, Statement};
use diesel::debug_query;
use diesel::pg::Pg;
use diesel::prelude::*;
use itertools::Itertools;

pub fn doc_by_iri<'a>(
    mut ctx: &'a mut DbContext,
    iri: &str,
) -> Result<(Document, HashModel), ErrorKind> {
    let docs = get_document(&mut ctx, iri);
    let first = docs.first();

    if docs.is_empty() {
        warn!(target: "apex", "Doc with iri '{}' is empty", iri);
        return Err(ErrorKind::EmptyDocument);
    }

    let mut props: HashModel = vec![];
    let (doc, resources) = first.unwrap();
    for (resource, resource_properties) in resources {
        for p in resource_properties {
            let predicate = ctx.property_map.iter().find(|(_, v)| **v == p.predicate_id);
            let predicate = match predicate {
                Some(s) => s.0,
                None => bail!(ErrorKind::Msg(format!("unknown predicate: {}", p.value))),
            };
            let datatype = ctx
                .datatype_map
                .iter()
                .find(|(_, v)| **v == p.datatype_id)
                .unwrap()
                .0;
            let empty = String::from(EMPTY_STRING);
            let language = match p.language_id {
                Some(id) => ctx.language_map.get_by_right(&id).unwrap_or(&empty),
                None => &empty,
            };

            props.push(Statement::new(
                ctx.lookup_table.ensure_value(&resource.iri),
                ctx.lookup_table.ensure_value(predicate),
                p.object_id.expect("Property without object_id").into(),
                ctx.lookup_table.ensure_value(datatype),
                ctx.lookup_table.ensure_value(language),
                ctx.lookup_table.ensure_value(&String::from(EMPTY_STRING)),
            ));
        }
    }

    Ok((doc.clone(), props))
}

const RANDOM_DOC_ID: &str = "SELECT *
FROM  (
    SELECT DISTINCT 1 + trunc(random() * 5100000)::integer AS id
    FROM   generate_series(1, 1100) g
    ) r
JOIN documents USING (id)
LIMIT  1;";

pub fn random_doc(ctx: &mut DbContext) -> Result<(Document, HashModel), ErrorKind> {
    let random_iri = match diesel::sql_query(RANDOM_DOC_ID).get_result::<Document>(&ctx.get_conn())
    {
        Ok(doc) => doc.iri,
        Err(e) => {
            warn!("{}", e);
            return Err(ErrorKind::NoResources);
        }
    };

    doc_by_iri(ctx, &random_iri)
}

const EMPTY_STRING: &str = "";

pub(crate) fn reset_document<'a>(mut ctx: &'a mut DbContext, iri: &str) -> (Document, HashModel) {
    match doc_by_iri(&mut ctx, iri) {
        Err(e) => {
            debug!("Error resetting document: {}", e);
            trace!("Document iri {} not yet in db", iri);
            let doc = &NewDocument {
                iri: String::from(iri),
                language: ctx.lang.clone().expect("No language given"),
            };
            let doc = diesel::insert_into(schema::documents::table)
                .values(doc)
                .get_result::<Document>(&ctx.get_conn())
                .expect("Error while inserting into documents");

            (doc, vec![])
        }
        Ok(model) => {
            trace!("Document with iri {} has id {}", model.0.iri, model.0.id);
            delete_document_data(&ctx.get_conn(), iri);
            model
        }
    }
}

pub(crate) fn update_cache_control(db_conn: &PgConnection, docs: &Vec<crate::models::Document>) {
    use schema::documents::dsl::*;

    for (cc, group) in &docs.into_iter().group_by(|d| d.cache_control) {
        let iris = group.map(|d| d.iri.clone()).collect::<Vec<String>>();
        let docs = documents.filter(iri.eq_any(iris));

        diesel::update(docs)
            .set(cache_control.eq(i16::from(cc)))
            .execute(db_conn)
            .unwrap();
    }
}

pub(crate) fn delete_all_document_data(db_conn: &PgConnection) -> QueryResult<usize> {
    db_conn.execute("TRUNCATE TABLE documents CASCADE")
}

pub(crate) fn delete_document_data(
    db_conn: &PgConnection,
    doc_iri: &str,
) -> Result<i64, ErrorKind> {
    use schema::documents;
    use schema::properties;
    use schema::resources::dsl::*;

    let doc_id = documents::table
        .filter(documents::iri.eq(doc_iri))
        .select(documents::id)
        .get_result::<i64>(db_conn)
        .map_err(|_| ErrorKind::NotFound)?;

    let resource_ids = resources
        .select(id)
        .filter(document_id.eq(doc_id))
        .get_results::<i64>(db_conn)
        .expect("Could not fetch resource ids for document");

    let props = properties::dsl::resource_id.eq_any(&resource_ids);
    diesel::delete(properties::table)
        .filter(props)
        .execute(db_conn)
        .expect("Couldn't delete existing properties");

    diesel::delete(resources)
        .filter(id.eq_any(&resource_ids))
        .execute(db_conn)
        .expect("Couldn't delete existing resources");

    Ok(doc_id)
}

fn get_document(
    db_ctx: &mut DbContext,
    doc_iri: &str,
) -> Vec<(Document, Vec<(Resource, Vec<Property>)>)> {
    use schema::documents::dsl::*;
    let db_conn = db_ctx.get_conn();

    let docs = if let Some(lang) = db_ctx.lang.clone() {
        documents
            .filter(iri.eq(doc_iri).and(language.eq(lang)))
            .load::<Document>(&db_conn)
            .unwrap()
    } else {
        documents
            .filter(iri.eq(doc_iri))
            .load::<Document>(&db_conn)
            .unwrap()
    };

    let doc_resources: Vec<Resource> = Resource::belonging_to(&docs)
        .load::<Resource>(&db_conn)
        .unwrap();

    let q = Property::belonging_to(&doc_resources);
    if cfg!(debug_assertions) {
        let sql = debug_query::<Pg, _>(&q).to_string();
        debug!(target: "apex", "Executing bulk query: {}", sql);
    }
    let doc_properties: Vec<Property> = match q.load::<Property>(&db_conn) {
        Ok(res) => res,
        Err(e) => {
            println!("{:?}", e);
            vec![]
        }
    };

    let object_ids = doc_properties
        .iter()
        .map(|p| p.object_id.expect("Property without object_id"))
        .collect::<Vec<_>>();

    let grouped_properties: Vec<Vec<Property>> = doc_properties.grouped_by(&doc_resources);

    let values = objects::objects
        .filter(objects::hash.eq_any(object_ids))
        .load::<Object>(&db_conn)
        .unwrap();

    values.iter().for_each(|object| {
        assert_eq!(
            db_ctx.lookup_table.ensure_value(&object.value),
            object.hash.into(),
            "Hash collision detected for value {}",
            object.value
        )
    });

    let resources_and_properties = doc_resources
        .into_iter()
        .zip(grouped_properties)
        .grouped_by(&docs);

    docs.into_iter().zip(resources_and_properties).collect()
}
