use std::path::Path;

use rusqlite::Connection as Conn;
use structopt::StructOpt;

use crate::builtins::utils::print_stderr_with_capture;
use crate::builtins::utils::print_stdout_with_capture;
use crate::ctime;
use crate::history;
use crate::parsers;
use crate::shell::Shell;
use crate::types::{CommandResult, CommandLine, Command};

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

pub fn run(sh: &mut Shell, cl: &CommandLine, cmd: &Command,
           capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    let hfile = history::get_history_file();
    let path = Path::new(hfile.as_str());
    if !path.exists() {
        let info = "no history file";
        print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
        return cr;
    }
    let conn = match Conn::open(&hfile) {
        Ok(x) => x,
        Err(e) => {
            let info = format!("history: sqlite error: {:?}", e);
            print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
            return cr;
        }
    };

    let tokens = cmd.tokens.clone();
    let args = parsers::parser_line::tokens_to_args(&tokens);

    let show_usage = args.len() > 1 && (args[1] == "-h" || args[1] == "--help");
    let opt = OptMain::from_iter_safe(args);
    match opt {
        Ok(opt) => {
            match opt.cmd {
                Some(SubCommand::Delete {rowid: rowids}) => {
                    let mut _count = 0;
                    for rowid in rowids {
                        let _deleted = delete_history_item(&conn, rowid);
                        if _deleted {
                            _count += 1;
                        }
                    }
                    if _count > 0 {
                        let info = format!("deleted {} items", _count);
                        print_stdout_with_capture(&info, &mut cr, cl, cmd, capture);
                    }
                    cr
                }
                Some(SubCommand::Add {timestamp: ts, input}) => {
                    let ts = ts.unwrap_or(0 as f64);
                    add_history(sh, ts, &input);
                    cr
                }
                None => {
                    let (str_out, str_err) = list_current_history(sh, &conn, &opt);
                    if !str_out.is_empty() {
                        print_stdout_with_capture(&str_out, &mut cr, cl, cmd, capture);
                    }
                    if !str_err.is_empty() {
                        print_stderr_with_capture(&str_err, &mut cr, cl, cmd, capture);
                    }
                    cr
                }
            }
        }
        Err(e) => {
            let info = format!("{}", e);
            if show_usage {
                print_stdout_with_capture(&info, &mut cr, cl, cmd, capture);
                cr.status = 0;
            } else {
                print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
                cr.status = 1;
            }
            cr
        }
    }
}

fn add_history(sh: &Shell, ts: f64, input: &str) {
    let (tsb, tse) = (ts, ts + 1.0);
    history::add_raw(sh, input, 0, tsb, tse);
}

fn list_current_history(sh: &Shell, conn: &Conn,
                        opt: &OptMain) -> (String, String) {
    let mut result_stderr = String::new();
    let result_stdout = String::new();

    let history_table = history::get_history_table();
    let mut sql = format!("SELECT ROWID, inp, tsb FROM {} WHERE ROWID > 0",
                          history_table);
    if !opt.pattern.is_empty() {
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
            let info = format!("history: prepare select error: {:?}", e);
            result_stderr.push_str(&info);
            return (result_stdout, result_stderr);
        }
    };

    let mut rows = match stmt.query([]) {
        Ok(x) => x,
        Err(e) => {
            let info = format!("history: query error: {:?}", e);
            result_stderr.push_str(&info);
            return (result_stdout, result_stderr);
        }
    };

    let mut lines = Vec::new();
    loop {
        match rows.next() {
            Ok(_rows) => {
                if let Some(row) = _rows {
                    let row_id: i32 = match row.get(0) {
                        Ok(x) => x,
                        Err(e) => {
                            let info = format!("history: error: {:?}", e);
                            result_stderr.push_str(&info);
                            return (result_stdout, result_stderr);
                        }
                    };
                    let inp: String = match row.get(1) {
                        Ok(x) => x,
                        Err(e) => {
                            let info = format!("history: error: {:?}", e);
                            result_stderr.push_str(&info);
                            return (result_stdout, result_stderr);
                        }
                    };

                    if opt.no_id {
                        lines.push(inp.to_string());
                    } else if opt.only_id {
                        lines.push(row_id.to_string());
                    } else if opt.show_date {
                        let tsb: f64 = match row.get(2) {
                            Ok(x) => x,
                            Err(e) => {
                                let info = format!("history: error: {:?}", e);
                                result_stderr.push_str(&info);
                                return (result_stdout, result_stderr);
                            }
                        };
                        let dt = ctime::DateTime::from_timestamp(tsb);
                        lines.push(format!("{}: {}: {}", row_id, dt, inp));
                    } else {
                        lines.push(format!("{}: {}", row_id, inp));
                    }
                } else {
                    break;
                }
            }
            Err(e) => {
                let info = format!("history: rows next error: {:?}", e);
                result_stderr.push_str(&info);
                return (result_stdout, result_stderr);
            }
        }
    }

    let buffer = lines.join("\n");

    (buffer, result_stderr)
}

fn delete_history_item(conn: &Conn, rowid: usize) -> bool {
    let history_table = history::get_history_table();
    let sql = format!("DELETE from {} where rowid = {}", history_table, rowid);
    match conn.execute(&sql, []) {
        Ok(_) => true,
        Err(e) => {
            log!("history: error when delete: {:?}", e);
            false
        }
    }
}
