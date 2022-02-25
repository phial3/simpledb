use anyhow::Result;

use super::{constant::Constant, updatescan::UpdateScan};

pub trait Scan {
    fn before_first(&mut self) -> Result<()>;
    fn next(&mut self) -> bool;
    fn get_i32(&mut self, fldname: &str) -> Result<i32>;
    fn get_string(&mut self, fldname: &str) -> Result<String>;
    fn get_val(&mut self, fldname: &str) -> Result<Constant>;
    fn has_field(&self, fldname: &str) -> bool;
    fn close(&mut self) -> Result<()>;

    fn to_update_scan(&mut self) -> Result<&mut dyn UpdateScan>;
}