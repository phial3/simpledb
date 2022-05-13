use std::{
    cell::RefCell,
    collections::HashMap,
    net::{SocketAddr, ToSocketAddrs},
    rc::Rc,
    sync::Arc,
};

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use itertools::Itertools;
use simpledb::{
    rdbc::{
        self,
        network::{
            metadata::NetworkResultSetMetaData, planrepr::NetworkPlanRepr, resultset::Value,
        },
        resultsetmetadataadapter::ResultSetMetaDataAdapter,
    },
    remote_capnp::{self, remote_connection, remote_driver, remote_result_set, remote_statement},
    repr::planrepr::{Operation, PlanRepr},
};

extern crate capnp_rpc;
extern crate simpledb;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:1099"
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");

    tokio::task::LocalSet::new().run_until(try_main(addr)).await
}

async fn get_server_version(
    driver: &remote_driver::Client,
) -> Result<(i32, i32), Box<dyn std::error::Error>> {
    let request = driver.get_version_request();
    let reply = request.send().promise.await?;
    let ver = reply.get()?.get_ver()?;

    Ok((ver.get_major_ver(), ver.get_minor_ver()))
}

async fn connect(
    driver: &remote_driver::Client,
    dbname: &str,
) -> Result<remote_connection::Client, Box<dyn std::error::Error>> {
    let mut request = driver.connect_request();
    request.get().set_dbname(dbname.into());
    let conn = request.send().pipeline.get_conn();

    Ok(conn)
}

async fn get_table_schema(
    conn: &remote_connection::Client,
    tblname: &str,
) -> Result<rdbc::network::metadata::Schema, Box<dyn std::error::Error>> {
    let mut schema = rdbc::network::metadata::Schema::new();

    let mut request = conn.get_table_schema_request();
    request.get().set_tblname(tblname.into());
    let reply = request.send().promise.await?;
    let sch = reply.get()?.get_sch()?;
    let fields = sch.get_fields()?;
    for i in 0..fields.len() {
        let fldname = fields.get(i as u32)?;
        schema.add_field(fldname);
    }

    let entries = sch.get_info()?.get_entries()?;
    for i in 0..entries.len() {
        let entry = entries.get(i as u32);
        let fldname = entry.get_key()?;
        let val = entry.get_value()?;
        match val.get_type()? {
            remote_capnp::FieldType::Integer => {
                let info = rdbc::network::metadata::FieldInfo::new_int32();
                schema.add_info(fldname, info);
            }
            remote_capnp::FieldType::Varchar => {
                let info =
                    rdbc::network::metadata::FieldInfo::new_string(val.get_length() as usize);
                schema.add_info(fldname, info);
            }
        }
    }

    Ok(schema)
}

async fn get_view_definition(
    conn: &remote_connection::Client,
    tblname: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut request = conn.get_view_definition_request();
    request.get().set_viewname(tblname.into());
    let reply = request.send().promise.await?;
    let viewdef = reply.get()?.get_vwdef()?;

    Ok((
        viewdef.reborrow().get_vwname()?.to_string(),
        viewdef.reborrow().get_vwdef()?.to_string(),
    ))
}

async fn get_index_info(
    conn: &remote_connection::Client,
    tblname: &str,
) -> Result<HashMap<String, rdbc::network::metadata::IndexInfo>, Box<dyn std::error::Error>> {
    let mut result = HashMap::new();

    let mut request = conn.get_index_info_request();
    request.get().set_tblname(tblname.into());
    let reply = request.send().promise.await?;
    let ii = reply.get()?.get_ii()?;
    let entries = ii.get_entries()?;
    for i in 0..entries.len() {
        let val = entries.get(i as u32).get_value()?;
        let fldname = val.get_fldname()?;
        let idxname = val.get_idxname()?;
        let info = rdbc::network::metadata::IndexInfo::new(fldname, idxname);
        result.insert(fldname.to_string(), info);
    }

    Ok(result)
}

struct NetworkResultSet {
    client: remote_result_set::Client,
}
impl NetworkResultSet {
    pub fn new(client: remote_result_set::Client) -> Self {
        Self { client }
    }

    pub async fn get_metadata(
        &self,
    ) -> Result<NetworkResultSetMetaData, Box<dyn std::error::Error>> {
        let meta_request = self.client.get_metadata_request();
        let meta_reply = meta_request.send().promise.await?;
        let meta = meta_reply.get()?.get_metadata()?;

        Ok(NetworkResultSetMetaData::from(meta))
    }
    pub async fn next(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let exists = self
            .client
            .next_request()
            .send()
            .promise
            .await?
            .get()?
            .get_exists();

        Ok(exists)
    }
    pub async fn get_row<'a, 'b: 'a>(
        &'a self,
        metadata: &'b NetworkResultSetMetaData,
    ) -> Result<HashMap<&'a str, Value>, Box<dyn std::error::Error>> {
        let request = self.client.get_row_request();
        let reply = request.send().promise.await?;
        let entry = to_hashmap(reply.get()?.get_row()?);

        let mut result = HashMap::new();
        for i in 0..metadata.get_column_count() {
            let fldname = metadata
                .get_column_name(i)
                .expect("get column name")
                .as_str();
            match entry.get(fldname) {
                Some(Value::Int32(v)) => {
                    result.insert(fldname, Value::Int32(*v));
                }
                Some(Value::String(s)) => {
                    result.insert(fldname, Value::String(s.clone()));
                }
                None => {
                    panic!("field missing");
                }
            }
        }
        Ok(result)
    }
}

async fn create_statement(
    conn: &remote_connection::Client,
    sql: &str,
) -> Result<remote_statement::Client, Box<dyn std::error::Error>> {
    let mut request = conn.create_statement_request();
    request.get().set_sql(sql.into());
    let stmt = request.send().pipeline.get_stmt();

    Ok(stmt)
}

async fn explain_plan(
    stmt: &remote_statement::Client,
) -> Result<NetworkPlanRepr, Box<dyn std::error::Error>> {
    let request = stmt.explain_plan_request();
    let reply = request.send().promise.await?;
    let planrepr = reply.get()?.get_planrepr()?;

    Ok(NetworkPlanRepr::from(planrepr))
}

fn format_name(op: Operation) -> String {
    match op {
        Operation::IndexJoinScan {
            idxname,
            idxfldname: _,
            joinfld: _,
        } => format!("{}", idxname),
        Operation::IndexSelectScan {
            idxname,
            idxfldname: _,
            val: _,
        } => format!("{}", idxname),
        Operation::GroupByScan {
            fields: _,
            aggfns: _,
        } => format!(""),
        Operation::Materialize => format!(""),
        Operation::MergeJoinScan {
            fldname1: _,
            fldname2: _,
        } => format!(""),
        Operation::SortScan { compflds: _ } => format!(""),
        Operation::MultibufferProductScan => format!(""),
        Operation::ProductScan => format!(""),
        Operation::ProjectScan => format!(""),
        Operation::SelectScan { pred: _ } => format!(""),
        Operation::TableScan { tblname } => format!("{}", tblname),
    }
}

fn format_operation(op: Operation) -> String {
    match op {
        Operation::IndexJoinScan {
            idxname: _,
            idxfldname,
            joinfld,
        } => format!("INDEX JOIN SCAN BY {} = {}", idxfldname, joinfld),
        Operation::IndexSelectScan {
            idxname: _,
            idxfldname,
            val,
        } => format!("INDEX SELECT SCAN BY {} = {}", idxfldname, val),
        Operation::GroupByScan {
            fields: _,
            aggfns: _,
        } => format!("GROUP BY",),
        Operation::Materialize => format!("MATERIALIZE"),
        Operation::MergeJoinScan { fldname1, fldname2 } => {
            format!("MERGE JOIN SCAN BY {} = {}", fldname1, fldname2)
        }
        Operation::SortScan { compflds } => format!("SORT SCAN BY ({})", compflds.iter().join(",")),
        Operation::MultibufferProductScan => format!("MULTIBUFFER PRODUCT SCAN"),
        Operation::ProductScan => format!("PRODUCT SCAN"),
        Operation::ProjectScan => format!("PROJECT SCAN"),
        Operation::SelectScan { pred: _ } => format!("SELECT SCAN"),
        Operation::TableScan { tblname: _ } => format!("TABLE SCAN"),
    }
}

fn print_explain_plan(pr: NetworkPlanRepr) {
    const MAX_OP_WIDTH: usize = 60;

    fn print_pr(pr: Arc<dyn PlanRepr>, n: Rc<RefCell<i32>>, depth: usize) {
        let raw_op_str = format_operation(pr.operation());
        let mut indented_op_str = format!("{:width$}{}", "", raw_op_str, width = depth * 2);
        if indented_op_str.len() > MAX_OP_WIDTH {
            // 3 is length of "..."
            indented_op_str = format!("{}...", &indented_op_str[0..MAX_OP_WIDTH - 3]);
        }
        println!(
            "{:>2} {:<width$} {:<20} {:>8} {:>8}",
            n.borrow(),
            indented_op_str,
            format_name(pr.operation()),
            pr.reads(),
            pr.writes(),
            width = MAX_OP_WIDTH,
        );
        *n.borrow_mut() += 1;

        for sub_pr in pr.sub_plan_reprs() {
            print_pr(sub_pr, Rc::clone(&n), depth + 1);
        }
    }

    let row_num = Rc::new(RefCell::new(1));
    let pr = pr.repr();
    println!(
        "{:<2} {:<width$} {:<20} {:>8} {:>8}",
        "#",
        "Operation",
        "Name",
        "Reads",
        "Writes",
        width = MAX_OP_WIDTH
    );
    println!("{:-<width$}", "", width = 102);
    print_pr(pr, row_num, 0);
}

async fn execute_query(
    stmt: &remote_statement::Client,
) -> Result<NetworkResultSet, Box<dyn std::error::Error>> {
    let client = stmt.execute_query_request().send().pipeline.get_result();

    Ok(NetworkResultSet::new(client))
}

async fn execute_command(
    stmt: &remote_statement::Client,
) -> Result<i32, Box<dyn std::error::Error>> {
    let reply = stmt.execute_update_request().send().promise.await?;
    let affected = reply.get()?.get_affected()?;
    let read = affected.read_request();
    let affected = read.send().promise.await?.get()?.get_affected();

    Ok(affected)
}

async fn try_main(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let stream = tokio::net::TcpStream::connect(&addr).await?;
    stream.set_nodelay(true)?;
    let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

    let rpc_network = Box::new(twoparty::VatNetwork::new(
        reader,
        writer,
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    ));

    let mut rpc_system = RpcSystem::new(rpc_network, None);
    let driver: remote_driver::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
    tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));

    // Query sample
    {
        if let Ok((major_ver, minor_ver)) = get_server_version(&driver).await {
            println!("simpledb server version {}.{}\n", major_ver, minor_ver);
        }

        let conn = connect(&driver, "demo").await?;

        // table schema
        let schema = get_table_schema(&conn, "student").await?;
        for fldname in schema.fields() {
            match schema.field_type(fldname.as_str()) {
                rdbc::network::metadata::FieldType::INTEGER => {
                    println!("{:10} {:10}", fldname, "INT32");
                }
                rdbc::network::metadata::FieldType::VARCHAR => {
                    println!(
                        "{:10} {:10}",
                        fldname,
                        format!("VARCHAR({})", schema.length(fldname))
                    );
                }
            }
        }
        println!();

        // index info
        let index_info = get_index_info(&conn, "student").await?;
        for (_, ii) in index_info.into_iter() {
            println!("{:20} {:10}", ii.index_name(), ii.field_name());
        }
        println!();

        // view definition
        let (vwname, vwdef) = get_view_definition(&conn, "einstein").await?;
        println!("view name: {}", vwname);
        println!("view def:  {}", vwdef);
        println!();

        let stmt = create_statement(
            &conn,
            "UPDATE student SET grad_year=2020 WHERE grad_year=2024",
        )
        .await?;
        let affected = execute_command(&stmt).await?;
        println!("Affected: {} rows", affected);

        // let commit_request = conn.commit_request();
        // commit_request.send().promise.await?;

        let stmt = create_statement(
            &conn,
            "SELECT sid, sname, dname, grad_year FROM student, dept WHERE did = major_id",
        )
        .await?;
        let plan = explain_plan(&stmt).await?;
        print_explain_plan(plan);
        println!();

        let mut result_set = execute_query(&stmt).await?;

        let metadata = result_set.get_metadata().await?;

        for i in 0..metadata.get_column_count() {
            let fldname = metadata
                .get_column_name(i)
                .expect("get column name")
                .as_str();
            let w = metadata
                .get_column_display_size(i)
                .expect("get column display size");
            print!("{:width$} ", fldname, width = w);
        }
        println!();
        for i in 0..metadata.get_column_count() {
            let w = metadata
                .get_column_display_size(i)
                .expect("get column display size");
            print!("{:-<width$}", "", width = w + 1);
        }
        println!();

        while result_set.next().await? {
            let entry = result_set.get_row(&metadata).await?;
            for i in 0..metadata.get_column_count() {
                let fldname = metadata
                    .get_column_name(i)
                    .expect("get column name")
                    .as_str();
                let w = metadata
                    .get_column_display_size(i)
                    .expect("get column display size");
                match entry.get(fldname) {
                    Some(Value::Int32(v)) => print!("{:width$} ", v, width = w),
                    Some(Value::String(s)) => print!("{:width$} ", s, width = w),
                    None => panic!("field missing"),
                }
            }
            println!();
        }

        let rollback_request = conn.rollback_request();
        rollback_request.send().promise.await?;
    }

    Ok(())
}

fn to_hashmap(row: remote_result_set::row::Reader) -> HashMap<&str, Value> {
    let entries = row.get_map().unwrap().get_entries().unwrap(); // TODO
    let mut result = HashMap::new();
    for kv in entries.into_iter() {
        let key = kv.get_key().unwrap(); // TODO
        let val = match kv.get_value().unwrap().which().unwrap() {
            remote_result_set::value::Int32(v) => Value::Int32(v),
            remote_result_set::value::String(s) => Value::String(s.unwrap().to_string()),
        };

        result.insert(key, val);
    }

    result
}
