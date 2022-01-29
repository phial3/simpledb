use anyhow::Result;
use core::fmt;
use std::{cell::RefCell, mem, sync::Arc};

use crate::{file::page::Page, log::manager::LogMgr};

use super::{LogRecord, TxType};

pub struct CheckpointRecord {}

impl fmt::Display for CheckpointRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<CHECKPOINT>")
    }
}

impl LogRecord for CheckpointRecord {
    fn op(&self) -> TxType {
        TxType::CHECKPOINT
    }
    fn tx_number(&self) -> i32 {
        -1 // dummy
    }
}
impl CheckpointRecord {
    pub fn new(p: Page) -> Result<Self> {
        Ok(Self {})
    }
    pub fn write_to_log(lm: Arc<RefCell<LogMgr>>) -> Result<u64> {
        let reclen = mem::size_of::<i32>();

        let mut p = Page::new_from_size(reclen);
        p.set_i32(0, TxType::CHECKPOINT as i32)?;

        lm.borrow_mut().append(p.contents())
    }
}
