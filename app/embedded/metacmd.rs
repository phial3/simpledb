use itertools::Itertools;
use std::{collections::HashMap, process};

use simpledb::{
    client::{
        explainplan::print_explain_plan, metacmd::print_help_meta_cmd,
        tableschema::print_table_schema, viewdef::print_view_definition,
    },
    rdbc::{
        connectionadapter::ConnectionAdapter, embedded::connection::EmbeddedConnection,
        model::IndexInfo,
    },
};

// TODO: make this common and move to simpledb::client
pub fn exec_meta_cmd(conn: &mut EmbeddedConnection, qry: &str) {
    let tokens: Vec<&str> = qry.trim().split_whitespace().collect_vec();
    let cmd = tokens[0].to_ascii_lowercase();
    let args = &tokens[1..];
    match cmd.as_str() {
        ":h" | ":help" => {
            print_help_meta_cmd();
        }
        ":q" | ":quit" | ":exit" => {
            conn.close().expect("close");
            println!("disconnected.");
            process::exit(0);
        }
        ":t" | ":table" => {
            if args.is_empty() {
                println!("table name is required.");
                return;
            }
            let tblname = args[0];
            if let Ok(sch) = conn.get_table_schema(tblname) {
                let idx_info: HashMap<String, IndexInfo> = conn
                    .get_index_info(tblname)
                    .unwrap_or_default()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone().into()))
                    .collect();
                print_table_schema(tblname, sch, idx_info);
            }
            return;
        }
        ":v" | ":view" => {
            if args.is_empty() {
                println!("view name is required.");
                return;
            }
            let viewname = args[0];
            if let Ok((viewname, viewdef)) = conn.get_view_definition(viewname) {
                print_view_definition(&viewname, &viewdef);
                println!();
            }
            return;
        }
        ":e" | ":explain" => {
            if args.is_empty() {
                println!("SQL is required.");
                return;
            }
            let sql = qry[tokens[0].len()..].trim();
            let mut stmt = conn.create_statement(sql).expect("create statement");
            let words: Vec<&str> = sql.split_whitespace().collect();
            if !words.is_empty() {
                let cmd = words[0].trim().to_ascii_lowercase();
                if &cmd == "select" {
                    if let Ok(plan_repr) = stmt.explain_plan() {
                        print_explain_plan(plan_repr.repr());
                        println!();
                        return;
                    }
                }
            }
            println!("expect query(not command).");
        }
        cmd => {
            println!("Unknown command: {}", cmd)
        }
    }
}
