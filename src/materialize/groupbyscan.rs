use std::sync::{Arc, Mutex};

use anyhow::Result;

use super::{aggregationfn::AggregationFn, groupvalue::GroupValue};
use crate::{
    query::{constant::Constant, scan::Scan, updatescan::UpdateScan},
    record::tablescan::TableScan,
};

pub struct GroupByScan {
    s: Arc<dyn Scan>,
    groupfields: Vec<String>,
    aggfns: Vec<Arc<dyn AggregationFn>>,
    groupval: GroupValue,
    moregroups: bool,
}

impl GroupByScan {
    pub fn new(
        s: Arc<Mutex<dyn Scan>>,
        groupfields: Vec<String>,
        aggfns: Vec<Arc<dyn AggregationFn>>,
    ) -> Self {
        panic!("TODO")
    }
}

impl Scan for GroupByScan {
    fn before_first(&mut self) -> Result<()> {
        panic!("TODO")
    }
    fn next(&mut self) -> bool {
        panic!("TODO")
    }
    fn get_i32(&mut self, fldname: &str) -> Result<i32> {
        panic!("TODO")
    }
    fn get_string(&mut self, fldname: &str) -> Result<String> {
        panic!("TODO")
    }
    fn get_val(&mut self, fldname: &str) -> Result<Constant> {
        panic!("TODO")
    }
    fn has_field(&self, fldname: &str) -> bool {
        panic!("TODO")
    }
    fn close(&mut self) -> Result<()> {
        panic!("TODO")
    }
    fn to_update_scan(&mut self) -> Result<&mut dyn UpdateScan> {
        panic!("TODO")
    }
    fn as_table_scan(&mut self) -> Result<&mut TableScan> {
        panic!("TODO")
    }
}
