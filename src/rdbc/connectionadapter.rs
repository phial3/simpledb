use anyhow::Result;
use core::fmt;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use super::statementadapter::StatementAdapter;
use crate::tx::transaction::Transaction;

#[derive(Debug)]
pub enum ConnectionError {
    CreateStatementFailed,
    StartNewTransactionFailed,
    CommitFailed,
    RollbackFailed,
    CloseFailed,
}

impl std::error::Error for ConnectionError {}
impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConnectionError::CreateStatementFailed => {
                write!(f, "failed to create statement")
            }
            ConnectionError::StartNewTransactionFailed => {
                write!(f, "failed to start new transaction")
            }
            ConnectionError::CommitFailed => {
                write!(f, "failed to commit")
            }
            ConnectionError::RollbackFailed => {
                write!(f, "failed to rollback")
            }
            ConnectionError::CommitFailed => {
                write!(f, "failed to commit")
            }
            ConnectionError::CloseFailed => {
                write!(f, "failed to close")
            }
        }
    }
}

pub trait ConnectionAdapter {
    fn create(&mut self, sql: &str) -> Result<Rc<RefCell<dyn StatementAdapter>>>;
    fn close(&mut self) -> Result<()>;
    fn commit(&mut self) -> Result<()>;
    fn rollback(&mut self) -> Result<()>;
    fn get_transaction(&self) -> Result<Arc<Mutex<Transaction>>>;
}