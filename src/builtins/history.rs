use std::io::Write;
use std::path::Path;

use chrono::{DateTime, NaiveDateTime, Local, Utc};
use rusqlite::Connection as Conn;
use rusqlite::NO_PARAMS;
use structopt::StructOpt;

use crate::history;
use crate::parsers;
use crate::shell;
use crate::types::Command;

#[derive(Debug, StructOpt)]
#[structopt(name = "history", about = "History in cicada shell")]
struct OptMain {
    #[structopt(short, long, help = "For current session only")]
    session: bool,

    #[structopt(short, long, help = "Search old items first")]
    asc: bool,

    #[structopt(short, long, help = "For current directory only")]
    pwd: bool,

    #[structopt(short, long, help = "Only show ROWID")]
    only_id: bool,

    #[structopt(short, long, help = "Do not show ROWID")]
    no_id: bool,

    #[structopt(short="d", long, help = "Show date")]
    show_date: bool,

    #[structopt(short, long, default_value = "20")]
    limit: i32,

    #[structopt(name = "PATTERN", default_value = "", help = "You can use % to match anything")]
    pattern: String,

    #[structopt(subcommand)]
    cmd: Option<SubCommand>
}

#[derive(StructOpt, Debug)]
enum SubCommand {
    #[structopt(about="Add new item into history")]
    Add {
        #[structopt(short="t", long, help = "Specify a timestamp for the new item")]
        timestamp: Option<f64>,

        #[structopt(name="INPUT", help = "input to be added into history")]
        input: String,
    },
    #[structopt(about="Delete item from history")]
    Delete {
        #[structopt(name="ROWID", help = "Row IDs of item to delete")]
        rowid: Vec<usize>,
    }
}

pub fn run(sh: &shell::Shell, cmd: &Command) -> i32 {
    let hfile = history::get_history_file();
    let path = Path::new(hfile.as_str());
    if !path.exists() {
        println_stderr!("no history file.");
        return 1;
    }
    let conn = match Conn::open(&hfile) {
        Ok(x) => x,
        Err(e) => {
            println!("sqlite conn open error: {:?}", e);
            return 1;
        }
    };

    let tokens = &cmd.tokens;
    let args = parsers::parser_line::tokens_to_args(tokens);

    let opt = OptMain::from_iter(args);
    match opt.cmd {
        Some(SubCommand::Delete {rowid: rowids}) => {
            for rowid in rowids {
                delete_history_item(&conn, rowid);
            }
            return 0;
        }
        Some(SubCommand::Add {timestamp: ts, input}) => {
            let ts = ts.unwrap_or(0 as f64);
            return add_history(sh, ts, &input);
        }
        None => {
            return list_current_history(sh, &conn, &opt);
        }
    }
}

fn add_history(sh: &shell::Shell, ts: f64, input: &str) -> i32 {
    let (tsb, tse) = (ts, ts + 1.0);
    history::add_raw(sh, input, 0, tsb, tse);
    0
}

fn list_current_history(sh: &shell::Shell, conn: &Conn, opt: &OptMain) -> i32 {
    let history_table = history::get_history_table();
    let mut sql = format!("SELECT ROWID, inp, tsb FROM {} WHERE ROWID > 0",
                          history_table);
    if opt.pattern.len() > 0 {
        sql = format!("{} AND inp LIKE '%{}%'", sql, opt.pattern)
    }
    if opt.session {
        sql = format!("{} AND sessionid = '{}'", sql, sh.session_id)
    }
    if opt.pwd {
        sql = format!("{} AND info like '%dir:{}|%'", sql, sh.current_dir)
    }

    if opt.asc {
        sql = format!("{} ORDER BY tsb", sql);
    } else {
        sql = format!("{} order by tsb desc", sql);
    };
    sql = format!("{} limit {} ", sql, opt.limit);

    let mut stmt = match conn.prepare(&sql) {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("history: prepare select error: {:?}", e);
            return 1;
        }
    };

    let mut rows = match stmt.query(NO_PARAMS) {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("history: query error: {:?}", e);
            return 1;
        }
    };

    loop {
        match rows.next() {
            Ok(_rows) => {
                if let Some(row) = _rows {
                    let row_id: i32 = match row.get(0) {
                        Ok(x) => x,
                        Err(e) => {
                            println_stderr!("history: error: {:?}", e);
                            return 1;
                        }
                    };
                    let inp: String = match row.get(1) {
                        Ok(x) => x,
                        Err(e) => {
                            println_stderr!("history: error: {:?}", e);
                            return 1;
                        }
                    };

                    if opt.no_id {
                        println!("{}", inp);
                    } else if opt.only_id {
                        println!("{}", row_id);
                    } else if opt.show_date {
                        let tsb: f64 = match row.get(2) {
                            Ok(x) => x,
                            Err(e) => {
                                println_stderr!("history: error: {:?}", e);
                                return 1;
                            }
                        };
                        let dt = DateTime::<Utc>::from_utc(
                            NaiveDateTime::from_timestamp(tsb as i64, 0), Utc
                        ).with_timezone(&Local);
                        println!("{}: {}: {}", row_id, dt.format("%Y-%m-%d %H:%M:%S"), inp);
                    } else {
                        println!("{}: {}", row_id, inp);
                    }
                } else {
                    return 0;
                }
            }
            Err(e) => {
                println_stderr!("history: rows next error: {:?}", e);
                return 1;
            }
        }
    }
}

fn delete_history_item(conn: &Conn, rowid: usize) {
    let history_table = history::get_history_table();
    let sql = format!("DELETE from {} where rowid = {}", history_table, rowid);
    match conn.execute(&sql, NO_PARAMS) {
        Ok(_) => {
            println!("item deleted.");
        }
        Err(e) => {
            println_stderr!("history: prepare error - {:?}", e);
        }
    }
}
