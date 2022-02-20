use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::{
    buffer::manager::BufferMgr,
    file::manager::FileMgr,
    log::manager::LogMgr,
    metadata::manager::MetadataMgr,
    tx::{concurrency::locktable::LockTable, transaction::Transaction},
};

pub const LOG_FILE: &str = "simpledb.log";
pub const BLOCK_SIZE: i32 = 400;
pub const BUFFER_SIZE: usize = 8;

pub struct SimpleDB {
    // configure
    db_directory: String,
    blocksize: i32,
    numbuffs: usize,

    // base for static members
    next_tx_num: Arc<Mutex<i32>>,
    locktbl: Arc<Mutex<LockTable>>,

    // managers
    fm: Arc<Mutex<FileMgr>>,
    lm: Arc<Mutex<LogMgr>>,
    bm: Arc<Mutex<BufferMgr>>,
    mdm: Option<Arc<Mutex<MetadataMgr>>>,
}

impl SimpleDB {
    pub fn new(db_directory: &str) -> Result<Self> {
        let mut db = SimpleDB::new_with(db_directory, LOG_FILE, BLOCK_SIZE, BUFFER_SIZE);
        let tx = Arc::new(Mutex::new(db.new_tx()?));
        let isnew = db.file_mgr().lock().unwrap().is_new();
        if isnew {
            println!("creating new database");
        } else {
            println!("recovering existing database");
            tx.lock().unwrap().recover()?;
        }
        db.mdm = Arc::new(Mutex::new(MetadataMgr::new(isnew, Arc::clone(&tx))?)).into();

        Ok(db)
    }
    pub fn new_with(db_directory: &str, logfile: &str, blocksize: i32, numbuffs: usize) -> Self {
        let next_tx_num = Arc::new(Mutex::new(0));
        let locktbl = Arc::new(Mutex::new(LockTable::new()));
        let fm = Arc::new(Mutex::new(
            FileMgr::new(&db_directory.clone(), blocksize).unwrap(),
        ));
        let lm = Arc::new(Mutex::new(LogMgr::new(Arc::clone(&fm), &logfile).unwrap()));
        let bm = Arc::new(Mutex::new(BufferMgr::new(
            Arc::clone(&fm),
            Arc::clone(&lm),
            numbuffs,
        )));

        Self {
            db_directory: db_directory.to_string(),
            blocksize,
            numbuffs,
            next_tx_num,
            locktbl,
            fm,
            lm,
            bm,
            mdm: None,
        }
    }
    pub fn file_mgr(&self) -> Arc<Mutex<FileMgr>> {
        Arc::clone(&self.fm)
    }
    pub fn log_mgr(&self) -> Arc<Mutex<LogMgr>> {
        Arc::clone(&self.lm)
    }
    pub fn buffer_mgr(&self) -> Arc<Mutex<BufferMgr>> {
        Arc::clone(&self.bm)
    }
    pub fn new_tx(&self) -> Result<Transaction> {
        Transaction::new(
            Arc::clone(&self.next_tx_num),
            Arc::clone(&self.locktbl),
            Arc::clone(&self.fm),
            Arc::clone(&self.lm),
            Arc::clone(&self.bm),
        )
    }
}
