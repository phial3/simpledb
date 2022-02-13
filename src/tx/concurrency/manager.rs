use anyhow::Result;

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex, Once},
};

use super::locktable::LockTable;
use crate::file::block_id::BlockId;

#[derive(Debug, Clone)]
pub struct ConcurrencyMgr {
    // static member (shared by all ConcurrentMgr)
    locktbl: Arc<Mutex<LockTable>>,

    locks: Rc<RefCell<HashMap<BlockId, String>>>,
}

impl ConcurrencyMgr {
    // emulate for static member locktbl
    pub fn new() -> Self {
        // make locktbl a static member by singleton pattern
        // ref.) https://stackoverflow.com/questions/27791532/how-do-i-create-a-global-mutable-singleton
        static mut LOCKTBL: Option<Arc<Mutex<LockTable>>> = None;
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                let locktbl = Arc::new(Mutex::new(LockTable::new()));
                LOCKTBL = Some(locktbl);
            });

            Self {
                locktbl: LOCKTBL.clone().unwrap(),
                locks: Rc::new(RefCell::new(HashMap::new())),
            }
        }
    }
    pub fn s_lock(&mut self, blk: &BlockId) -> Result<()> {
        if self.locks.borrow().get(blk).is_none() {
            self.locktbl.lock().unwrap().s_lock(blk)?;
            self.locks.borrow_mut().insert(blk.clone(), "S".to_string());
        }

        Ok(())
    }
    pub fn x_lock(&mut self, blk: &BlockId) -> Result<()> {
        if !self.has_x_lock(blk) {
            self.s_lock(blk)?;
            self.locktbl.lock().unwrap().x_lock(blk)?;
            self.locks.borrow_mut().insert(blk.clone(), "X".to_string());
        }

        Ok(())
    }
    pub fn release(&mut self) -> Result<()> {
        for blk in self.locks.borrow().keys() {
            self.locktbl.lock().unwrap().unlock(blk)?;
        }
        self.locks.borrow_mut().clear();

        Ok(())
    }
    fn has_x_lock(&self, blk: &BlockId) -> bool {
        if let Some(locktype) = self.locks.borrow().get(blk) {
            return locktype.eq("X");
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::manager::BufferMgr;
    use crate::file::manager::FileMgr;
    use crate::log::manager::LogMgr;
    use crate::tx::transaction::Transaction;

    use anyhow::Result;
    use std::path::Path;
    use std::time::Duration;
    use std::{fs, thread};

    #[test]
    fn unit_test() -> Result<()> {
        if Path::new("_concurrencytest").exists() {
            fs::remove_dir_all("_concurrencytest")?;
        }

        let fm = Arc::new(Mutex::new(FileMgr::new("_concurrencytest", 400)?));
        let lm = Arc::new(Mutex::new(LogMgr::new(Arc::clone(&fm), "testfile")?));
        let bm = Arc::new(Mutex::new(BufferMgr::new(
            Arc::clone(&fm),
            Arc::clone(&lm),
            8,
        )));

        let fm_a = Arc::clone(&fm);
        let lm_a = Arc::clone(&lm);
        let bm_a = Arc::clone(&bm);
        let handle1 = thread::spawn(|| {
            let mut tx_a = Transaction::new(fm_a, lm_a, bm_a);
            let blk1 = BlockId::new("testfile", 1);
            let blk2 = BlockId::new("testfile", 2);
            tx_a.pin(&blk1).unwrap();
            tx_a.pin(&blk2).unwrap();
            println!("Tx A: request slock 1");
            tx_a.get_i32(&blk1, 0);
            println!("Tx A: receive slock 1");
            thread::sleep(Duration::new(1, 0));
            println!("Tx A: request slock 2");
            tx_a.get_i32(&blk2, 0);
            println!("Tx A: receive slock 2");
            tx_a.commit().unwrap();
        });

        let fm_b = Arc::clone(&fm);
        let lm_b = Arc::clone(&lm);
        let bm_b = Arc::clone(&bm);
        let handle2 = thread::spawn(|| {
            let mut tx_b = Transaction::new(fm_b, lm_b, bm_b);
            let blk1 = BlockId::new("testfile", 1);
            let blk2 = BlockId::new("testfile", 2);
            tx_b.pin(&blk1).unwrap();
            tx_b.pin(&blk2).unwrap();
            println!("Tx B: request xlock 2");
            tx_b.set_i32(&blk2, 0, 0, false);
            println!("Tx B: receive xlock 2");
            thread::sleep(Duration::new(1, 0));
            println!("Tx B: request slock 1");
            tx_b.get_i32(&blk1, 0);
            println!("Tx B: receive slock 1");
            tx_b.commit().unwrap();
        });

        let fm_c = Arc::clone(&fm);
        let lm_c = Arc::clone(&lm);
        let bm_c = Arc::clone(&bm);
        let handle3 = thread::spawn(|| {
            let mut tx_c = Transaction::new(fm_c, lm_c, bm_c);
            let blk1 = BlockId::new("testfile", 1);
            let blk2 = BlockId::new("testfile", 2);
            tx_c.pin(&blk1).unwrap();
            tx_c.pin(&blk2).unwrap();
            println!("Tx C: request xlock 1");
            tx_c.set_i32(&blk1, 0, 0, false);
            println!("Tx C: receive xlock 1");
            thread::sleep(Duration::new(1, 0));
            println!("Tx C: request slock 2");
            tx_c.get_i32(&blk2, 0);
            println!("Tx C: receive slock 2");
            tx_c.commit().unwrap();
        });

        handle1.join().unwrap();
        handle2.join().unwrap();
        handle3.join().unwrap();

        Ok(())
    }
}
