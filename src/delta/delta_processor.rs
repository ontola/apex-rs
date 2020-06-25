use crate::hashtuple::{HashModel, Statement};

pub trait DeltaProcessor<'a> {
    fn matches(&self, statement: Statement) -> bool;
    fn process(
        &self,
        current: &HashModel,
        delta: &HashModel,
        statement: Statement,
    ) -> (HashModel, HashModel, HashModel);
}
