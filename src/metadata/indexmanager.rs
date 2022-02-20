use anyhow::Result;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    index::{hash::HashIndex, Index},
    record::{layout::Layout, schema::FieldType, schema::Schema, tablescan::TableScan},
    tx::transaction::Transaction,
};

use super::{
    statmanager::{StatInfo, StatMgr},
    tablemanager::{TableMgr, MAX_NAME},
};

#[derive(Debug, Clone)]
pub struct IndexMgr {
    layout: Layout,
    tblmgr: TableMgr,
    statmgr: StatMgr,
}

impl IndexMgr {
    pub fn new(
        isnew: bool,
        tblmgr: TableMgr,
        statmgr: StatMgr,
        tx: Arc<Mutex<Transaction>>,
    ) -> Result<Self> {
        let mgr = Self {
            layout: tblmgr.get_layout("idxcat", Arc::clone(&tx))?,
            tblmgr,
            statmgr,
        };

        if isnew {
            let mut sch = Schema::new();
            sch.add_string_field("indexname", MAX_NAME);
            sch.add_string_field("tablename", MAX_NAME);
            sch.add_string_field("fieldname", MAX_NAME);
            mgr.tblmgr.create_table("idxcat", sch, tx)?;
        }

        Ok(mgr)
    }
    pub fn create_index(
        &self,
        idxname: &str,
        tblname: &str,
        fldname: &str,
        tx: Arc<Mutex<Transaction>>,
    ) -> Result<()> {
        let mut ts = TableScan::new(tx, "idxcat", self.layout.clone())?;
        ts.insert()?;
        ts.set_string("indexname", idxname.to_string())?;
        ts.set_string("tablename", tblname.to_string())?;
        ts.set_string("fieldname", fldname.to_string())?;
        ts.close()?;

        Ok(())
    }
    pub fn get_index_info(
        &mut self,
        tblname: &str,
        tx: Arc<Mutex<Transaction>>,
    ) -> Result<HashMap<String, IndexInfo>> {
        let mut result = HashMap::new();
        let mut ts = TableScan::new(Arc::clone(&tx), "idxcat", self.layout.clone())?;
        while ts.next() {
            if ts.get_string("tablename")? == tblname {
                let idxname = ts.get_string("indexname")?;
                let fldname: String = ts.get_string("fieldname")?;
                let tbl_layout = self.tblmgr.get_layout(tblname, Arc::clone(&tx))?;
                let tblsi =
                    self.statmgr
                        .get_stat_info(&tblname, tbl_layout.clone(), Arc::clone(&tx))?;
                let ii = IndexInfo::new(
                    idxname,
                    fldname.clone(),
                    tbl_layout.schema().clone(),
                    Arc::clone(&tx),
                    tblsi,
                );
                result.insert(fldname, ii);
            }
        }
        ts.close()?;

        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct IndexInfo {
    idxname: String,
    fldname: String,
    tx: Arc<Mutex<Transaction>>,
    tbl_schema: Schema,
    idx_layout: Layout,
    si: StatInfo,
}

impl IndexInfo {
    pub fn new(
        idxname: String,
        fldname: String,
        tbl_schema: Schema,
        tx: Arc<Mutex<Transaction>>,
        si: StatInfo,
    ) -> Self {
        let sch = Schema::new();
        let layout = Layout::new(sch);

        let mut mgr = Self {
            idxname,
            fldname,
            tx,
            tbl_schema,
            idx_layout: layout, // dummy
            si,
        };

        mgr.idx_layout = mgr.create_idx_layout();

        mgr
    }
    pub fn open(&self) -> Arc<Mutex<dyn Index>> {
        let idx = HashIndex::new(
            Arc::clone(&self.tx),
            self.idxname.clone(),
            self.idx_layout.clone(),
        );

        Arc::new(Mutex::new(idx))
    }
    pub fn blocks_accessed(&self) -> i32 {
        let rpb = self.tx.lock().unwrap().block_size() / self.idx_layout.slot_size() as i32;
        let numblocks = self.si.records_output() / rpb;
        HashIndex::search_cost(numblocks, rpb)
    }
    pub fn records_output(&self) -> i32 {
        self.si.records_output() / self.si.distinct_values(&self.fldname)
    }
    pub fn distinct_values(&self, fname: &str) -> i32 {
        if self.fldname == fname {
            return 1;
        } else {
            return self.si.distinct_values(&self.fldname);
        }
    }
    fn create_idx_layout(&mut self) -> Layout {
        let mut sch = Schema::new();
        sch.add_i32_field("block");
        sch.add_i32_field("id");
        match self.tbl_schema.field_type(&self.fldname) {
            FieldType::INTEGER => {
                sch.add_i32_field("dataval");
            }
            FieldType::VARCHAR => {
                let fldlen = self.tbl_schema.length(&self.fldname);
                sch.add_string_field("dataval", fldlen);
            }
        }

        Layout::new(sch)
    }
}
