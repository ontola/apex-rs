use crate::delta::delta_processor::DeltaProcessor;
use crate::delta::processor::ProcessorInitializer;
use crate::hashtuple::{HashModel, LookupTable, Statement};

const ONT_INVALIDATE: &str = "https://ns.ontola.io/invalidate";

pub struct InvalidateProcessor<'a> {
    pub(crate) lookup_table: &'a LookupTable,
}

impl<'a> ProcessorInitializer for InvalidateProcessor<'a> {
    fn initialize(lookup_table: &mut LookupTable) {
        lookup_table.ensure_value(&String::from(ONT_INVALIDATE));
    }
}

/// Currently assumes the form <sub> <*> <*> <ont:inv>
impl<'a> DeltaProcessor<'a> for InvalidateProcessor<'a> {
    #[rustfmt::skip]
    fn matches(&self, statement: Statement) -> bool {
        let graph = statement.graph;

        graph == self.lookup_table.get_by_value(String::from(ONT_INVALIDATE))
    }

    fn process(
        &self,
        cur: &HashModel,
        _: &HashModel,
        st: Statement,
    ) -> (HashModel, HashModel, HashModel) {
        let var = self
            .lookup_table
            .get_by_value(String::from("http://spinrdf.org/sp#Variable"));
        if st.subject == var || st.predicate != var || st.value != var {
            error!(
                "Processor only supports <subj> <sp:Variable> <sp:Variable> <ont:inv> statements"
            );
        }
        let replaces = Vec::with_capacity(0);
        let removes = cur.clone();

        (Vec::with_capacity(0), replaces, removes)
    }
}
