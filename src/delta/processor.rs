use crate::delta::delta_processor::DeltaProcessor;
use crate::delta::processors::add_processor::AddProcessor;
use crate::delta::processors::replace_processor::ReplaceProcessor;
use crate::delta::processors::supplant_processor::SupplantProcessor;
use crate::hashtuple::{HashModel, LookupTable, Statement};
use crate::importing::events::DeltaProcessingTiming;
use crate::serving::serialization::{
    hash_model_to_hextuples, hashtuple_to_hextuple, hextuple_to_utf8,
};
use std::time::Instant;

pub(crate) trait ProcessorInitializer {
    fn initialize(lookup_table: &mut LookupTable);
}

pub fn default_processors<'a>(
    lookup_table: &'a LookupTable,
) -> Vec<Box<dyn DeltaProcessor<'a> + 'a>> {
    vec![
        Box::new(SupplantProcessor::<'a> { lookup_table }),
        Box::new(AddProcessor::<'a> { lookup_table }),
        Box::new(ReplaceProcessor::<'a> { lookup_table }),
        //         RemoveProcessor {},
        //         PurgeProcessor {},
        //         SliceProcessor {},
    ]
}

pub fn add_processor_methods_to_table(lookup_table: &mut LookupTable) {
    AddProcessor::initialize(lookup_table);
    ReplaceProcessor::initialize(lookup_table);
    SupplantProcessor::initialize(lookup_table);
}

/// FIXME: Be sure to call `add_processor_methods_to_table` on the `lookup_table` beforehand.
pub fn apply_delta(
    lookup_table: &LookupTable,
    current: &HashModel,
    delta: &HashModel,
) -> (HashModel, DeltaProcessingTiming) {
    let mut timing = DeltaProcessingTiming::new();
    let setup_start = Instant::now();
    let processors = default_processors(lookup_table);

    let mut result = current.clone();
    let mut addable: HashModel = vec![];
    let mut replaceable: HashModel = vec![];
    let mut removable: HashModel = vec![];

    let setup_end = Instant::now();
    timing.setup_time = setup_end.duration_since(setup_start);

    for statement in delta {
        match processors.iter().find(|p| p.matches(*statement)) {
            Some(processor) => {
                let (adds, replaces, removes) = processor.process(current, &delta, *statement);

                addable.extend(adds);
                replaceable.extend(replaces);
                removable.extend(removes);
            }
            None => {
                let hex = String::from_utf8(hextuple_to_utf8(&hashtuple_to_hextuple(
                    statement,
                    &lookup_table,
                )))
                .unwrap();
                warn!(target: "apex", "No processor for statement, discarding: {}", hex);
            }
        }
    }
    let sort_end = Instant::now();
    timing.sort_time = sort_end.duration_since(setup_end);

    let remove_end = Instant::now();
    if !removable.is_empty() {
        result = remove_all(&result, &removable);
    }
    timing.remove_time = remove_end.duration_since(sort_end);

    if !replaceable.is_empty() {
        result = replace_matches(&mut result, &replaceable);
    }
    let replace_end = Instant::now();
    timing.replace_time = replace_end.duration_since(remove_end);

    if !addable.is_empty() {
        add_all(&mut result, &addable);
    }
    timing.add_time = Instant::now().duration_since(replace_end);

    trace!(target: "apex", "Result after delta: {}", String::from_utf8(hash_model_to_hextuples((result.clone(), &lookup_table))).unwrap());

    (result, timing)
}

fn remove_all(cur: &HashModel, patch: &HashModel) -> HashModel {
    let mut next = vec![];
    for h in cur {
        if !contains(patch, h) {
            next.push(*h);
        }
    }

    next
}

fn replace_matches(cur: &mut HashModel, patch: &HashModel) -> HashModel {
    let mut cleaned: HashModel = Vec::with_capacity(patch.len());
    for st in patch {
        match cur
            .iter()
            .find(|x| x.subject == st.subject && x.predicate == st.predicate)
        {
            Some(patch_value) => cleaned.push(*patch_value),
            None => cleaned.push(*st),
        }
    }

    cleaned
}

fn add_all(cur: &mut HashModel, patch: &HashModel) {
    let mut matches = patch
        .into_iter()
        .filter(|h| !contains(&cur, h))
        .cloned()
        .collect::<Vec<Statement>>();

    cur.append(matches.as_mut());
}

fn contains(model: &HashModel, h: &Statement) -> bool {
    model.iter().any(|x| x == h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashtuple::STRING_IRI;

    #[test]
    fn test_apply_delta_with_replace() {
        let mut lookup_table = LookupTable::new(1);
        add_processor_methods_to_table(&mut lookup_table);
        let named_node = lookup_table.ensure_value(&String::from("rdf:namedNode"));
        let string = lookup_table.ensure_value(&String::from(STRING_IRI));
        let replace =
            lookup_table.ensure_value(&String::from("http://purl.org/linked-delta/replace"));

        let name = lookup_table.ensure_value(&String::from("https://schema.org/name"));
        let homepage = lookup_table.ensure_value(&String::from("https://schema.org/homepage"));
        let comment = lookup_table.ensure_value(&String::from("https://schema.org/comment"));

        let id = lookup_table.ensure_value(&String::from("https://id.openraadsinformatie.nl/1234"));
        let bob = lookup_table.ensure_value(&String::from("bob"));
        let bob_corrected = lookup_table.ensure_value(&String::from("Bob"));
        let empty = lookup_table.ensure_value(&String::from(""));
        let bobs_homepage = lookup_table.ensure_value(&String::from("https://bob.com"));
        let comment0 = lookup_table.ensure_value(&String::from("Comment 0"));
        let comment1 = lookup_table.ensure_value(&String::from("Comment 1"));

        let cur: HashModel = vec![Statement::new(id, name, bob, string, empty, empty)];
        let patch: HashModel = vec![
            Statement::new(id, name, bob_corrected, string, empty, replace),
            Statement::new(id, homepage, bobs_homepage, named_node, empty, replace),
            Statement::new(id, comment, comment0, string, empty, replace),
            Statement::new(id, comment, comment1, named_node, empty, replace),
        ];

        let (out, _) = apply_delta(&mut lookup_table, &cur, &patch);

        assert_eq!(out.len(), 4);
        assert_eq!(cur.len(), 1);
        assert_eq!(patch.len(), 4);
    }

    #[test]
    fn test_add_all() {
        let mut cur: HashModel = vec![Statement::new(2u128, 0u128, 0u128, 0u128, 0u128, 0u128)];
        let patch: HashModel = vec![
            Statement::new(0u128, 0u128, 0u128, 0u128, 0u128, 0u128),
            Statement::new(1u128, 0u128, 0u128, 0u128, 0u128, 0u128),
            Statement::new(2u128, 0u128, 0u128, 0u128, 0u128, 0u128),
            Statement::new(3u128, 0u128, 0u128, 0u128, 0u128, 0u128),
        ];

        add_all(&mut cur, &patch);

        assert_eq!(cur.len(), 4)
    }
}
