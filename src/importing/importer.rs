use crate::db::db_context::DbContext;
use crate::db::document::{delete_all_document_data, reset_document};
use crate::db::properties::insert_properties;
use crate::db::resources::insert_resources;
use crate::delta::processor::{add_processor_methods_to_table, apply_delta};
use crate::errors::ErrorKind;
use crate::importing::events::{DeltaProcessingTiming, MessageTiming};
use crate::importing::parsing::DocumentSet;
use diesel::prelude::*;
use diesel::result::Error::RollbackTransaction;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[allow(unused_must_use)]
pub(crate) async fn process_message(
    ctx: &mut DbContext<'_>,
    docs: DocumentSet,
) -> Result<MessageTiming, ErrorKind> {
    let mut result: Result<MessageTiming, ErrorKind> = Err(ErrorKind::Unexpected);

    ctx.db_pool
        .get()
        .unwrap()
        .transaction::<(), diesel::result::Error, _>(|| match process_delta(ctx, docs) {
            Ok(timing) => {
                result = Ok(timing);

                Ok(())
            }
            Err(e) => {
                result = Err(e);
                Err(RollbackTransaction)
            }
        });

    result
}

#[allow(unused_must_use)]
pub(crate) async fn process_invalidate(
    ctx: &mut DbContext<'_>,
) -> Result<MessageTiming, ErrorKind> {
    debug!(target: "apex", "Invalidating all data");
    let mut result: Result<MessageTiming, ErrorKind> = Err(ErrorKind::Unexpected);

    ctx.db_pool
        .get()
        .unwrap()
        .transaction(|| match delete_all_document_data(&ctx.get_conn()) {
            Ok(_) => {
                result = Ok(MessageTiming::new());
                Ok(())
            }
            Err(e) => {
                result = Err(ErrorKind::Unexpected);
                Err(RollbackTransaction)
            }
        });

    result
}

pub(crate) fn process_delta(
    mut ctx: &mut DbContext,
    docs: DocumentSet,
) -> Result<MessageTiming, ErrorKind> {
    let parse_start = Instant::now();

    add_processor_methods_to_table(&mut ctx.lookup_table);

    let parse_time = Instant::now().duration_since(parse_start);
    let mut fetch_time = Duration::new(0, 0);
    let mut delta_time = DeltaProcessingTiming::new();
    let mut insert_time = Duration::new(0, 0);

    for (iri, delta) in docs {
        let fetch_start = Instant::now();

        let (existing_doc, existing_model) = reset_document(&mut ctx, &iri);
        fetch_time += Instant::now().duration_since(fetch_start);

        let (next, delta_timing) = apply_delta(&ctx.lookup_table, &existing_model, &delta);
        delta_time += delta_timing;

        let insert_start = Instant::now();
        let resources =
            insert_resources(&ctx.get_conn(), &ctx.lookup_table, &next, existing_doc.id);
        let mut resource_id_map = HashMap::<String, i64>::new();
        for resource in resources {
            resource_id_map.insert(resource.iri.clone(), resource.id);
        }

        insert_properties(ctx, &next, resource_id_map);
        insert_time += Instant::now().duration_since(insert_start);
    }

    Ok(MessageTiming {
        poll_time: Duration::new(0, 0),
        parse_time,
        fetch_time,
        delta_time,
        insert_time,
    })
}
