use crate::delta::delta_processor::DeltaProcessor;
use crate::delta::processor::ProcessorInitializer;
use crate::hashtuple::{HashModel, LookupTable, Statement};

const LD_SUPPLANT: &str = "http://purl.org/linked-delta/supplant";
const LL_SUPPLANT: &str = "http://purl.org/link-lib/supplant";

pub struct SupplantProcessor<'a> {
    pub(crate) lookup_table: &'a LookupTable,
}

impl<'a> ProcessorInitializer for SupplantProcessor<'a> {
    fn initialize(lookup_table: &mut LookupTable) {
        lookup_table.ensure_value(&String::from(LD_SUPPLANT));
        lookup_table.ensure_value(&String::from(LL_SUPPLANT));
    }
}

impl<'a> DeltaProcessor<'a> for SupplantProcessor<'a> {
    #[rustfmt::skip]
    fn matches(&self, statement: Statement) -> bool {
        let graph = statement.graph;

        graph == self.lookup_table.get_by_value(String::from(LD_SUPPLANT))
            || graph == self.lookup_table.get_by_value(String::from(LL_SUPPLANT))
    }

    fn process(
        &self,
        cur: &HashModel,
        _: &HashModel,
        st: Statement,
    ) -> (HashModel, HashModel, HashModel) {
        let replaces = vec![st];
        let removes = cur.clone();

        (Vec::with_capacity(0), replaces, removes)
    }
}
