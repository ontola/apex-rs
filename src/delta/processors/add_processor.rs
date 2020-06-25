use crate::delta::delta_processor::DeltaProcessor;
use crate::delta::processor::ProcessorInitializer;
use crate::hashtuple::{HashModel, LookupTable, Statement};

const LD_ADD: &str = "http://purl.org/linked-delta/add";

pub struct AddProcessor<'a> {
    pub(crate) lookup_table: &'a LookupTable,
}

impl<'a> ProcessorInitializer for AddProcessor<'a> {
    fn initialize(lookup_table: &mut LookupTable) {
        lookup_table.ensure_value(&String::from(LD_ADD));
    }
}

impl<'a> DeltaProcessor<'a> for AddProcessor<'a> {
    fn matches(&self, statement: Statement) -> bool {
        statement.graph == self.lookup_table.get_by_value(String::from(LD_ADD))
    }

    fn process(
        &self,
        _: &HashModel,
        _: &HashModel,
        st: Statement,
    ) -> (HashModel, HashModel, HashModel) {
        let adds = vec![st];
        let replaces = Vec::with_capacity(0);
        let removes = Vec::with_capacity(0);

        (adds, replaces, removes)
    }
}
