use capnp::capability::Promise;
use capnp_rpc::pry;
use log::{debug, info, trace};
use std::sync::{Arc, Mutex};

use super::simpledb::SimpleDB;
use crate::{
    remote_capnp::{self, remote_connection, remote_driver, remote_statement},
    tx::transaction::Transaction,
};

const MAJOR_VERSION: i32 = 0;
const MINOR_VERSION: i32 = 1;

pub trait Server {
    fn get_database(&mut self, dbname: &str) -> Arc<Mutex<SimpleDB>>;
}

pub struct RemoteDriverImpl {
    major_ver: i32,
    minor_ver: i32,
    server: Arc<Mutex<dyn Server>>,
}

impl RemoteDriverImpl {
    pub fn new(srv: Arc<Mutex<dyn Server>>) -> Self {
        Self {
            major_ver: MAJOR_VERSION,
            minor_ver: MINOR_VERSION,
            server: srv,
        }
    }
}

impl remote_driver::Server for RemoteDriverImpl {
    fn connect(
        &mut self,
        params: remote_driver::ConnectParams,
        mut results: remote_driver::ConnectResults,
    ) -> Promise<(), capnp::Error> {
        trace!("connecting");
        let dbname = pry!(pry!(params.get()).get_dbname());
        info!("connect db: {}", dbname);
        let db = self.server.lock().unwrap().get_database(dbname);
        let conn: remote_connection::Client =
            capnp_rpc::new_client(RemoteConnectionImpl::new(dbname, db));
        results.get().set_conn(conn);
        trace!("connected");

        Promise::ok(())
    }
    fn get_version(
        &mut self,
        _: remote_driver::GetVersionParams,
        mut results: remote_driver::GetVersionResults,
    ) -> Promise<(), capnp::Error> {
        trace!("get version");
        results.get().init_ver().set_major_ver(self.major_ver);
        results.get().init_ver().set_minor_ver(self.minor_ver);
        info!("version: {}.{}", self.major_ver, self.minor_ver);

        Promise::ok(())
    }
}

pub struct RemoteConnectionImpl {
    dbname: String,
    db: Arc<Mutex<SimpleDB>>,
    current_tx: Arc<Mutex<Transaction>>,
}

impl RemoteConnectionImpl {
    pub fn new(dbname: &str, db: Arc<Mutex<SimpleDB>>) -> Self {
        let tx = db.lock().unwrap().new_tx().expect("new transaction");
        debug!("tx: {}", tx.tx_num());

        Self {
            dbname: dbname.to_string(),
            db,
            current_tx: Arc::new(Mutex::new(tx)),
        }
    }
}

impl remote_capnp::remote_connection::Server for RemoteConnectionImpl {
    fn create_statement(
        &mut self,
        params: remote_connection::CreateStatementParams,
        mut results: remote_connection::CreateStatementResults,
    ) -> Promise<(), capnp::Error> {
        let sql = pry!(pry!(params.get()).get_sql());
        trace!("SQL: {}", sql);
        let stmt: remote_statement::Client = capnp_rpc::new_client(RemoteStatementImpl::new(sql));
        results.get().set_stmt(stmt);

        Promise::ok(())
    }
    fn close(
        &mut self,
        _: remote_capnp::remote_connection::CloseParams,
        _: remote_capnp::remote_connection::CloseResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
    fn commit(
        &mut self,
        _: remote_capnp::remote_connection::CommitParams,
        _: remote_capnp::remote_connection::CommitResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
    fn rollback(
        &mut self,
        _: remote_capnp::remote_connection::RollbackParams,
        _: remote_capnp::remote_connection::RollbackResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
    fn get_table_schema(
        &mut self,
        _: remote_capnp::remote_connection::GetTableSchemaParams,
        _: remote_capnp::remote_connection::GetTableSchemaResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
    fn get_view_definition(
        &mut self,
        _: remote_capnp::remote_connection::GetViewDefinitionParams,
        _: remote_capnp::remote_connection::GetViewDefinitionResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
    fn get_index_info(
        &mut self,
        _: remote_capnp::remote_connection::GetIndexInfoParams,
        _: remote_capnp::remote_connection::GetIndexInfoResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
}

pub struct RemoteStatementImpl {
    sql: String,
}
impl RemoteStatementImpl {
    pub fn new(sql: &str) -> Self {
        Self {
            sql: sql.to_string(),
        }
    }
}

impl remote_capnp::remote_statement::Server for RemoteStatementImpl {
    fn execute_query(
        &mut self,
        _: remote_capnp::remote_statement::ExecuteQueryParams,
        _: remote_capnp::remote_statement::ExecuteQueryResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
    fn execute_update(
        &mut self,
        _: remote_capnp::remote_statement::ExecuteUpdateParams,
        _: remote_capnp::remote_statement::ExecuteUpdateResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
    fn close(
        &mut self,
        _: remote_capnp::remote_statement::CloseParams,
        _: remote_capnp::remote_statement::CloseResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
    fn explain_plan(
        &mut self,
        _: remote_capnp::remote_statement::ExplainPlanParams,
        _: remote_capnp::remote_statement::ExplainPlanResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
}

pub struct RemoteResultSetImpl;

impl remote_capnp::remote_result_set::Server for RemoteResultSetImpl {
    fn next(
        &mut self,
        _: remote_capnp::remote_result_set::NextParams,
        _: remote_capnp::remote_result_set::NextResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
    fn close(
        &mut self,
        _: remote_capnp::remote_result_set::CloseParams,
        _: remote_capnp::remote_result_set::CloseResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
    fn get_metadata(
        &mut self,
        _: remote_capnp::remote_result_set::GetMetadataParams,
        _: remote_capnp::remote_result_set::GetMetadataResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
    fn get_next_record(
        &mut self,
        _: remote_capnp::remote_result_set::GetNextRecordParams,
        _: remote_capnp::remote_result_set::GetNextRecordResults,
    ) -> Promise<(), capnp::Error> {
        panic!("TODO")
    }
}
