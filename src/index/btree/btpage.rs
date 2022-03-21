use anyhow::Result;
use core::fmt;
use std::{
    mem,
    sync::{Arc, Mutex},
};

use crate::{
    file::block_id::BlockId,
    query::constant::Constant,
    record::{layout::Layout, rid::RID, schema::FieldType},
    tx::transaction::Transaction,
};

#[derive(Debug)]
pub enum BTPageError {
    NoCurrentBlockError,
}

impl std::error::Error for BTPageError {}
impl fmt::Display for BTPageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BTPageError::NoCurrentBlockError => {
                write!(f, "no current block")
            }
        }
    }
}

pub struct BTPage {
    tx: Arc<Mutex<Transaction>>,
    currentblk: Option<BlockId>,
    layout: Arc<Layout>,
}

impl BTPage {
    pub fn new(
        tx: Arc<Mutex<Transaction>>,
        currentblk: BlockId,
        layout: Arc<Layout>,
    ) -> Result<Self> {
        tx.lock().unwrap().pin(&currentblk)?;

        Ok(Self {
            tx,
            currentblk: Some(currentblk),
            layout,
        })
    }
    pub fn find_slot_before(&self, searchkey: Constant) -> Result<i32> {
        let mut slot = 0;

        while slot < self.get_num_recs()? && self.get_data_val(slot)? < searchkey {
            slot += 1;
        }

        Ok(slot - 1)
    }
    pub fn close(&mut self) -> Result<()> {
        match &self.currentblk {
            Some(currentblk) => {
                self.tx.lock().unwrap().unpin(&currentblk)?;
                self.currentblk = None;
                Ok(())
            }
            None => Err(From::from(BTPageError::NoCurrentBlockError)),
        }
    }
    pub fn is_full(&self) -> bool {
        self.slotpos(self.get_num_recs().unwrap() + 1) >= self.tx.lock().unwrap().block_size()
    }
    pub fn split(&mut self, splitpos: i32, flag: i32) -> Result<BlockId> {
        let newblk = self.append_new(flag)?;
        let mut newpage = BTPage::new(
            Arc::clone(&self.tx),
            newblk.clone(),
            Arc::clone(&self.layout),
        )?;
        self.transfer_recs(splitpos, &mut newpage)?;
        newpage.set_flag(flag)?;
        newpage.close()?;

        Ok(newblk)
    }
    pub fn get_data_val(&self, slot: i32) -> Result<Constant> {
        self.get_val(slot, "dataval")
    }
    pub fn get_flag(&self) -> Result<i32> {
        match self.currentblk.as_ref() {
            Some(currentblk) => self.tx.lock().unwrap().get_i32(currentblk, 0),
            None => Err(From::from(BTPageError::NoCurrentBlockError)),
        }
    }
    pub fn set_flag(&mut self, val: i32) -> Result<()> {
        match self.currentblk.as_ref() {
            Some(currentblk) => self.tx.lock().unwrap().set_i32(currentblk, 0, val, true),
            None => Err(From::from(BTPageError::NoCurrentBlockError)),
        }
    }
    pub fn append_new(&mut self, flag: i32) -> Result<BlockId> {
        match self.currentblk.as_ref() {
            Some(currentblk) => {
                let blk = self.tx.lock().unwrap().append(&currentblk.file_name())?;
                self.tx.lock().unwrap().pin(&blk)?;
                self.format(&blk, flag)?;

                Ok(blk)
            }
            None => Err(From::from(BTPageError::NoCurrentBlockError)),
        }
    }
    pub fn format(&mut self, blk: &BlockId, flag: i32) -> Result<()> {
        let mut tx = self.tx.lock().unwrap();
        tx.set_i32(blk, 0, flag, false)?;
        tx.set_i32(blk, mem::size_of::<i32>() as i32, 0, false)?; // #records = 0
        let recsize = self.layout.slot_size() as i32;
        let mut pos = 2 * mem::size_of::<i32>() as i32;
        while pos + recsize <= tx.block_size() {
            self.make_default_record(blk, pos)?;
            pos += recsize;
        }

        Ok(())
    }
    pub fn make_default_record(&self, blk: &BlockId, pos: i32) -> Result<()> {
        for fldname in self.layout.schema().fields() {
            let offset = self.layout.offset(fldname) as i32;
            match self.layout.schema().field_type(fldname) {
                FieldType::INTEGER => {
                    let mut tx = self.tx.lock().unwrap();
                    tx.set_i32(blk, pos + offset, 0, false)?;
                }
                FieldType::VARCHAR => {
                    let mut tx = self.tx.lock().unwrap();
                    tx.set_string(blk, pos + offset, "", false)?;
                }
            }
        }
        Ok(())
    }
    // Methods called only by BTreeDir
    pub fn get_child_num(&self, slot: i32) -> Result<i32> {
        self.get_i32(slot, "block")
    }
    pub fn insert_dir(&mut self, slot: i32, val: Constant, blknum: i32) -> Result<()> {
        self.insert(slot)?;
        self.set_val(slot, "dataval", val)?;
        self.set_i32(slot, "block", blknum)
    }
    // Methods called only by BTreeLeaf
    pub fn get_data_rid(&self, slot: i32) -> Result<RID> {
        Ok(RID::new(
            self.get_i32(slot, "block")?,
            self.get_i32(slot, "id")?,
        ))
    }
    pub fn insert_leaf(&mut self, slot: i32, val: Constant, rid: RID) -> Result<()> {
        self.insert(slot)?;
        self.set_val(slot, "dataval", val)?;
        self.set_i32(slot, "block", rid.block_number())?;
        self.set_i32(slot, "id", rid.slot())
    }

    pub fn delete(&mut self, slot: i32) -> Result<()> {
        for i in slot + 1..self.get_num_recs()? {
            self.copy_record(i, i - 1)?;
        }
        self.set_num_recs(self.get_num_recs()? - 1)
    }
    pub fn get_num_recs(&self) -> Result<i32> {
        match &self.currentblk {
            Some(currentblk) => {
                let mut tx = self.tx.lock().unwrap();
                tx.get_i32(currentblk, mem::size_of::<i32>() as i32)
            }
            None => Err(From::from(BTPageError::NoCurrentBlockError)),
        }
    }
    // Private methods
    fn get_i32(&self, slot: i32, fldname: &str) -> Result<i32> {
        match &self.currentblk {
            Some(currentblk) => {
                let pos = self.fldpos(slot, fldname);
                self.tx.lock().unwrap().get_i32(currentblk, pos)
            }
            None => Err(From::from(BTPageError::NoCurrentBlockError)),
        }
    }
    fn get_string(&self, slot: i32, fldname: &str) -> Result<String> {
        match &self.currentblk {
            Some(currentblk) => {
                let pos = self.fldpos(slot, fldname);
                self.tx.lock().unwrap().get_string(currentblk, pos)
            }
            None => Err(From::from(BTPageError::NoCurrentBlockError)),
        }
    }
    fn get_val(&self, slot: i32, fldname: &str) -> Result<Constant> {
        let fldtype = self.layout.schema().field_type(fldname);
        match fldtype {
            FieldType::INTEGER => Ok(Constant::new_i32(self.get_i32(slot, fldname)?)),
            FieldType::VARCHAR => Ok(Constant::new_string(self.get_string(slot, fldname)?)),
        }
    }
    fn set_i32(&mut self, slot: i32, fldname: &str, val: i32) -> Result<()> {
        match &self.currentblk {
            Some(currentblk) => {
                let pos = self.fldpos(slot, fldname);
                let mut tx = self.tx.lock().unwrap();
                tx.set_i32(currentblk, pos, val, true)
            }
            None => Err(From::from(BTPageError::NoCurrentBlockError)),
        }
    }
    fn set_string(&mut self, slot: i32, fldname: &str, val: &str) -> Result<()> {
        match &self.currentblk {
            Some(currentblk) => {
                let pos = self.fldpos(slot, fldname);
                let mut tx = self.tx.lock().unwrap();
                tx.set_string(currentblk, pos, val, true)
            }
            None => Err(From::from(BTPageError::NoCurrentBlockError)),
        }
    }
    fn set_val(&mut self, slot: i32, fldname: &str, val: Constant) -> Result<()> {
        let fldtype = self.layout.schema().field_type(fldname);
        match fldtype {
            FieldType::INTEGER => self.set_i32(slot, fldname, val.as_i32()?),
            FieldType::VARCHAR => self.set_string(slot, fldname, val.as_string()?),
        }
    }
    fn set_num_recs(&mut self, n: i32) -> Result<()> {
        match &self.currentblk {
            Some(currentblk) => {
                let mut tx = self.tx.lock().unwrap();
                tx.set_i32(currentblk, mem::size_of::<i32>() as i32, n, true)
            }
            None => Err(From::from(BTPageError::NoCurrentBlockError)),
        }
    }
    fn insert(&mut self, slot: i32) -> Result<()> {
        let mut i = self.get_num_recs()?;
        while i > slot {
            self.copy_record(i - 1, i)?;
            i -= 1;
        }
        self.set_num_recs(self.get_num_recs()? + 1)
    }
    fn copy_record(&mut self, from: i32, to: i32) -> Result<()> {
        let sch = self.layout.schema();
        for fldname in sch.fields() {
            self.set_val(to, fldname, self.get_val(from, fldname)?)?;
        }

        Ok(())
    }
    fn transfer_recs(&mut self, slot: i32, dest: &mut BTPage) -> Result<()> {
        let mut destslot = 0;
        while slot < self.get_num_recs()? {
            dest.insert(destslot)?;
            let sch = self.layout.schema();
            for fldname in sch.fields() {
                dest.set_val(destslot, fldname, self.get_val(slot, fldname)?)?;
            }
            self.delete(slot)?;
            destslot += 1;
        }

        Ok(())
    }
    fn fldpos(&self, slot: i32, fldname: &str) -> i32 {
        let offset = self.layout.offset(fldname) as i32;
        self.slotpos(slot) + offset
    }
    fn slotpos(&self, slot: i32) -> i32 {
        let slotsize = self.layout.slot_size() as i32;
        2 * mem::size_of::<i32>() as i32 + (slot * slotsize)
    }
}
