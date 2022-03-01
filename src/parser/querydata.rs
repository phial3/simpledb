use core::fmt;
use std::borrow::Borrow;

use itertools::Itertools;

use crate::query::predicate::Predicate;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueryData {
    fields: Vec<String>,
    tables: Vec<String>,
    pred: Predicate,
}

impl fmt::Display for QueryData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut result = vec![];
        result.push("select");
        let mut fs = vec![];
        for fldname in self.fields.iter() {
            fs.push(fldname.as_str());
        }
        let fs_str = fs.join(", ");
        result.push(fs_str.as_str());
        result.push("from");
        let mut ts = vec![];
        for tblname in self.tables.iter() {
            ts.push(tblname.as_str())
        }
        let ts_str = ts.join(", ");
        result.push(ts_str.as_str());
        result.push("where");
        let pred_str = self.pred.to_string();
        result.push(&pred_str.as_str());

        write!(f, "{}", result.join(" "))
    }
}

impl QueryData {
    pub fn new(fields: Vec<String>, tables: Vec<String>, pred: Predicate) -> Self {
        Self {
            fields,
            tables,
            pred,
        }
    }
    pub fn fields(&self) -> Vec<&String> {
        self.fields.iter().collect()
    }
    pub fn tables(&self) -> Vec<&String> {
        self.tables.iter().collect()
    }
    pub fn pred(&self) -> &Predicate {
        &self.pred
    }
}
