use crate::types::{Hextuple, Model};

trait DeltaProcessor {
    fn matches(&self, statement: &Hextuple) -> bool;
    fn process(
        &self,
        current: &Model,
        delta: &Model,
        statement: &Hextuple,
    ) -> (Model, Model, Model);
}

fn default_processors<'a>() -> Vec<&'a DeltaProcessor> {
    vec![
//         SupplantProcessor {},
        &AddProcessor {},
        &ReplaceProcessor {},
//         RemoveProcessor {},
//         PurgeProcessor {},
//         SliceProcessor {},
    ]
}

pub fn apply_delta(current: &Model, delta: &Model) -> Model {
    let processors = default_processors();

    let mut result = current.clone();
    let mut addable: Model = vec![];
    let mut replaceable: Model = vec![];
    let mut removable: Model = vec![];

    for statement in delta {
        if let processor = &processors.iter().find(|p| p.matches(&statement)) {
            let (adds, replaces, removes) = processor
                .expect("No processor for delta")
                .process(current, &delta, &statement);

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

fn remove_all(cur: &Model, patch: &Model) -> Model {
    let mut next = vec![];
    for h in cur {
        if !contains(patch, h) {
            next.push(h.clone());
        }
    }

    next
}

fn replace_matches(cur: &mut Model, patch: &Model) -> Model {
    let mut cleaned = vec![];
    for st in cur {
        match patch.iter().find(|x| x[0] == st[0] && x[1] == st[1]) {
            Some(patch_value) => cleaned.push(patch_value.clone()),
            None => cleaned.push(st.clone()),
        }
    }

    cleaned
}

fn add_all(cur: &mut Model, patch: &Model) {
    for h in patch {
        if !contains(&cur, &h) {
            cur.push(h.clone())
        }
    }
}

fn contains(model: &Model, h: &Hextuple) -> bool {
    model.iter().find(|x| equals(x, h)).is_some()
}

fn equals(a: &Hextuple, b: &Hextuple) -> bool {
    a[0] == b[0] && a[1] == b[1] && a[2] == b[2] && a[3] == b[3] && a[4] == b[4] && a[5] == b[5]
}

fn hex_without_context(hex: &Hextuple) -> Hextuple {
    [hex[0].clone(), hex[1].clone(), hex[2].clone(), hex[3].clone(), hex[4].clone(), String::from("rdf:defaultGraph")]
}

// struct SupplantProcessor {}

struct AddProcessor {}
impl DeltaProcessor for AddProcessor {
    fn matches(&self, statement: &Hextuple) -> bool {
        statement[5] == String::from("http://purl.org/linked-delta/add")
    }

    fn process(
        &self,
        current: &Model,
        delta: &Model,
        statement: &Hextuple,
    ) -> (Model, Model, Model) {
        let adds: Model = delta.clone();
        let replaces: Model = vec![];
        let removes: Model = vec![];

        (adds, replaces, removes)
    }
}

struct ReplaceProcessor {}
impl DeltaProcessor for ReplaceProcessor {
    fn matches(&self, statement: &Hextuple) -> bool {
        statement[5] == String::from("http://purl.org/linked-delta/replace")
            || statement[5] == String::from("http://purl.org/linked-delta/supplant")
            || statement[5] == String::from("http://purl.org/link-lib/supplant")
    }

    fn process(
            &self,
            current: &Model,
            delta: &Model,
            st: &Hextuple,
        ) -> (Model, Model, Model) {
            let adds = delta.clone();
            let copy = [
                st[0].clone(),
                st[1].clone(),
                st[2].clone(),
                st[3].clone(),
                st[4].clone(),
                st[5].clone(),
            ];
            let replaces: Model = vec![copy];

            (adds, replaces, vec![])
        }
}

// struct RemoveProcessor {}
//
// struct PurgeProcessor {}
//
// struct SliceProcessor {}
