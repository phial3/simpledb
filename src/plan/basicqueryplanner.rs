use anyhow::Result;
use combine::Parser;
use std::sync::{Arc, Mutex};

use super::plan::Plan;
use crate::{
    metadata::manager::MetadataMgr,
    parser::{
        parser::{create_view, query},
        querydata::QueryData,
    },
    plan::{
        productplan::ProductPlan, projectplan::ProjectPlan, selectplan::SelectPlan,
        tableplan::TablePlan,
    },
    tx::transaction::Transaction,
};

pub struct BasicQueryPlanner {
    mdm: MetadataMgr,
}

impl BasicQueryPlanner {
    pub fn new(mdm: MetadataMgr) -> Self {
        Self { mdm }
    }
    pub fn create_plan(&mut self, data: QueryData, tx: Arc<Mutex<Transaction>>) -> Arc<dyn Plan> {
        // Step 1: Create a plan for each mentioned table or view
        let mut plans: Vec<Arc<dyn Plan>> = vec![];
        for tblname in data.tables() {
            let viewdef = self.mdm.get_view_def(tblname, Arc::clone(&tx)).unwrap();
            if !viewdef.is_empty() {
                // Recursively plan the view.
                let mut parser = query();
                let (viewdata, _) = parser.parse(viewdef.as_str()).unwrap();
                plans.push(self.create_plan(viewdata, Arc::clone(&tx)));
            } else {
                plans.push(Arc::new(
                    TablePlan::new(Arc::clone(&tx), tblname, self.mdm.clone()).unwrap(),
                ))
            }
        }
        // Step 2: Create the product of all table plans
        let mut p = plans.remove(0);
        for nextplan in plans {
            p = Arc::new(ProductPlan::new(Arc::clone(&p), nextplan));
        }
        // Step 3: Add a selection plan for the predicate
        p = Arc::new(SelectPlan::new(Arc::clone(&p), data.pred().clone()));

        // Step 4: Project on the field names
        Arc::new(ProjectPlan::new(p, data.fields().clone()))
    }
}