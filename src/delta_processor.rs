use crate::hashtuple::{HashModel, Hashtuple, LookupTable};

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
pub fn apply_delta<'a>(
    lookup_table: &'a LookupTable,
    current: &HashModel,
    delta: &HashModel,
) -> HashModel {
    let processors = default_processors(lookup_table);

    let mut result = current.clone();
    let mut addable: HashModel = vec![];
    let mut replaceable: HashModel = vec![];
    let mut removable: HashModel = vec![];

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

    result = remove_all(&result, &removable);
    result = replace_matches(&mut result, &replaceable);
    add_all(&mut result, &addable);

    result
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
    for st in cur {
        match patch.iter().find(|x| x[0] == st[0] && x[1] == st[1]) {
            Some(patch_value) => cleaned.push(patch_value.clone()),
            None => cleaned.push(*st),
        }
    }

    cleaned
}

fn add_all(cur: &mut HashModel, patch: &HashModel) {
    for h in patch {
        if !contains(&cur, h) {
            cur.push(*h)
        }
    }
}

fn contains(model: &HashModel, h: &Hashtuple) -> bool {
    model.iter().find(|x| equals(x, h)).is_some()
}

fn equals(a: &Hashtuple, b: &Hashtuple) -> bool {
    a[0] == b[0] && a[1] == b[1] && a[2] == b[2] && a[3] == b[3] && a[4] == b[4] && a[5] == b[5]
}

// fn hex_without_context(hex: &Hashtuple) -> Hashtuple {
//     [
//         hex[0],
//         hex[1],
//         hex[2],
//         hex[3],
//         hex[4],
//         String::from("rdf:defaultGraph"),
//     ]
// }

// struct SupplantProcessor {}

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
        delta: &HashModel,
        _: Hashtuple,
    ) -> (HashModel, HashModel, HashModel) {
        let adds = delta.clone();
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
        delta: &HashModel,
        st: Hashtuple,
    ) -> (HashModel, HashModel, HashModel) {
        let adds = delta.clone();
        let replaces = vec![st];

        (adds, replaces, Vec::with_capacity(0))
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
