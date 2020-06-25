use crate::delta::delta_processor::DeltaProcessor;
use crate::delta::processor::ProcessorInitializer;
use crate::hashtuple::{HashModel, LookupTable, Statement};

const LD_REPLACE: &str = "http://purl.org/linked-delta/replace";

pub struct ReplaceProcessor<'a> {
    pub(crate) lookup_table: &'a LookupTable,
}

impl<'a> ProcessorInitializer for ReplaceProcessor<'a> {
    fn initialize(lookup_table: &mut LookupTable) {
        lookup_table.ensure_value(&String::from(LD_REPLACE));
    }
}

impl<'a> DeltaProcessor<'a> for ReplaceProcessor<'a> {
    #[rustfmt::skip]
    fn matches(&self, statement: Statement) -> bool {
        let graph = statement.graph;

        graph == self.lookup_table.get_by_value(String::from(LD_REPLACE))
    }

    fn process(
        &self,
        _: &HashModel,
        _: &HashModel,
        st: Statement,
    ) -> (HashModel, HashModel, HashModel) {
        let replaces = vec![st];

        (Vec::with_capacity(0), replaces, Vec::with_capacity(0))
    }
}
