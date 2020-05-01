use crate::events::DeltaProcessingTiming;
use crate::hashtuple::{HashModel, Hashtuple, LookupTable};
use std::time::Instant;

const LD_ADD: &str = "http://purl.org/linked-delta/add";
const LD_REPLACE: &str = "http://purl.org/linked-delta/replace";
const LD_SUPPLANT: &str = "http://purl.org/linked-delta/supplant";
const LL_SUPPLANT: &str = "http://purl.org/link-lib/supplant";

pub trait DeltaProcessor<'a> {
    fn matches(&self, statement: Hashtuple) -> bool;
    fn process(
        &self,
        current: &HashModel,
        delta: &HashModel,
        statement: Hashtuple,
    ) -> (HashModel, HashModel, HashModel);
}

trait ProcessorInitializer {
    fn initialize(lookup_table: &mut LookupTable);
}

pub fn default_processors<'a>(
    lookup_table: &'a LookupTable,
) -> Vec<Box<dyn DeltaProcessor<'a> + 'a>> {
    vec![
        //         SupplantProcessor {},
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
        let processor = processors.iter().find(|p| p.matches(*statement));
        if processor.is_some() {
            let (adds, replaces, removes) = processor
                .expect("No processor for delta")
                .process(current, &delta, *statement);

            addable.extend(adds);
            replaceable.extend(replaces);
            removable.extend(removes);
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

/// TODO: reverse
fn replace_matches(cur: &mut HashModel, patch: &HashModel) -> HashModel {
    let mut cleaned: HashModel = Vec::with_capacity(patch.len());
    for st in patch {
        match cur.iter().find(|x| x[0] == st[0] && x[1] == st[1]) {
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
        .collect::<Vec<Hashtuple>>();

    cur.append(matches.as_mut());
}

fn contains(model: &HashModel, h: &Hashtuple) -> bool {
    model.iter().any(|x| equals(x, h))
}

fn equals(a: &Hashtuple, b: &Hashtuple) -> bool {
    a[0] == b[0] && a[1] == b[1] && a[2] == b[2] && a[3] == b[3] && a[4] == b[4] && a[5] == b[5]
}

struct AddProcessor<'a> {
    lookup_table: &'a LookupTable,
}
impl<'a> DeltaProcessor<'a> for AddProcessor<'a> {
    fn matches(&self, statement: Hashtuple) -> bool {
        statement[5] == self.lookup_table.get_by_value(String::from(LD_ADD))
    }

    fn process(
        &self,
        _: &HashModel,
        _: &HashModel,
        st: Hashtuple,
    ) -> (HashModel, HashModel, HashModel) {
        let adds = vec![st];
        let replaces = Vec::with_capacity(0);
        let removes = Vec::with_capacity(0);

        (adds, replaces, removes)
    }
}
impl<'a> ProcessorInitializer for AddProcessor<'a> {
    fn initialize(lookup_table: &mut LookupTable) {
        lookup_table.ensure_value(&String::from(LD_ADD));
    }
}

struct ReplaceProcessor<'a> {
    lookup_table: &'a LookupTable,
}
impl<'a> DeltaProcessor<'a> for ReplaceProcessor<'a> {
    #[rustfmt::skip]
    fn matches(&self, statement: Hashtuple) -> bool {
        let graph = statement[5];

        graph == self.lookup_table.get_by_value(String::from(LD_REPLACE))
            || graph == self.lookup_table.get_by_value(String::from(LD_SUPPLANT))
            || graph == self.lookup_table.get_by_value(String::from(LL_SUPPLANT))
    }

    fn process(
        &self,
        _: &HashModel,
        _: &HashModel,
        st: Hashtuple,
    ) -> (HashModel, HashModel, HashModel) {
        let replaces = vec![st];

        (Vec::with_capacity(0), replaces, Vec::with_capacity(0))
    }
}
impl<'a> ProcessorInitializer for ReplaceProcessor<'a> {
    fn initialize(lookup_table: &mut LookupTable) {
        lookup_table.ensure_value(&String::from(LD_REPLACE));
        lookup_table.ensure_value(&String::from(LD_SUPPLANT));
        lookup_table.ensure_value(&String::from(LL_SUPPLANT));
    }
}

// struct RemoveProcessor {}
//
// struct PurgeProcessor {}
//
// struct SliceProcessor {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_delta_with_replace() {
        let mut lookup_table = LookupTable::new();
        add_processor_methods_to_table(&mut lookup_table);
        let named_node = lookup_table.ensure_value(&String::from("rdf:namedNode"));
        let string = lookup_table.ensure_value(&String::from("xsd:string"));
        let replace = lookup_table.ensure_value(&String::from(LD_REPLACE));

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

        let cur: HashModel = vec![[id, name, bob, string, empty, empty]];
        let patch: HashModel = vec![
            [id, name, bob_corrected, string, empty, replace],
            [id, homepage, bobs_homepage, named_node, empty, replace],
            [id, comment, comment0, string, empty, replace],
            [id, comment, comment1, named_node, empty, replace],
        ];

        let (out, _) = apply_delta(&mut lookup_table, &cur, &patch);

        assert_eq!(out.len(), 4);
        assert_eq!(cur.len(), 1);
        assert_eq!(patch.len(), 4);
    }

    #[test]
    fn test_add_all() {
        let mut cur: HashModel = vec![[2u128, 0u128, 0u128, 0u128, 0u128, 0u128]];
        let patch: HashModel = vec![
            [0u128, 0u128, 0u128, 0u128, 0u128, 0u128],
            [1u128, 0u128, 0u128, 0u128, 0u128, 0u128],
            [2u128, 0u128, 0u128, 0u128, 0u128, 0u128],
            [3u128, 0u128, 0u128, 0u128, 0u128, 0u128],
        ];

        add_all(&mut cur, &patch);

        assert_eq!(cur.len(), 4)
    }
}
