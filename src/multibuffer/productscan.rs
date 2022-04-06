use core::panic;
use std::sync::{Arc, Mutex};

use anyhow::Result;

use crate::{
    materialize::sortscan::SortScan,
    query::{constant::Constant, scan::Scan, updatescan::UpdateScan},
    record::{layout::Layout, tablescan::TableScan},
    tx::transaction::Transaction,
};

pub struct MultibufferProductScan {
    tx: Arc<Mutex<Transaction>>,
    lhsscan: Arc<Mutex<dyn Scan>>,
    rhsscan: Option<Arc<Mutex<dyn Scan>>>,
    prodscan: Arc<Mutex<dyn Scan>>,
    filename: String,
    layout: Arc<Layout>,
    chunksize: i32,
    nextblknum: i32,
    filesize: i32,
}

impl MultibufferProductScan {
    pub fn new(
        tx: Arc<Mutex<Transaction>>,
        lhsscan: Arc<Mutex<dyn Scan>>,
        filename: &str,
        layout: Arc<Layout>,
    ) -> Self {
        panic!("TODO")
    }
}

impl Scan for MultibufferProductScan {
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
    fn as_sort_scan(&mut self) -> Result<&mut SortScan> {
        panic!("TODO")
    }
}
