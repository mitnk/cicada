use std::io::Write;
use std::path::Path;

use rusqlite::Connection as Conn;
use rusqlite::NO_PARAMS;

use crate::history;
use crate::types;

pub fn run(cmd: &types::Command) -> i32 {
    let tokens = &cmd.tokens;

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

    if tokens.len() == 1 {
        return list_current_history(&conn);
    } else if tokens.len() == 2 {
        search_history(&conn, &tokens[1].1);
    } else if tokens.len() == 3 {
        if tokens[1].1 != "delete" {
            println_stderr!("only support: history delete");
            return 1;
        }

        if let Ok(rowid) = tokens[2].1.parse::<usize>() {
            delete_history_item(&conn, rowid);
        } else {
            println_stderr!("history delete: a row number is needed");
            return 1;
        }
    } else {
        println_stderr!("history: only take one or two args");
        return 1;
    }
    0
}

fn list_current_history(conn: &Conn) -> i32 {
    let history_table = history::get_history_table();
    let sql = format!(
        "SELECT rowid, inp FROM {} ORDER BY tsb desc limit 20;",
        history_table
    );
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
                    println!("{}: {}", row_id, inp);
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

fn search_history(conn: &Conn, q: &str) {
    let history_table = history::get_history_table();
    let sql = format!(
        "SELECT ROWID, inp FROM {}
             WHERE inp like '%{}%'
             ORDER BY tsb desc limit 50;",
        history_table, q
    );
    let mut stmt = match conn.prepare(&sql) {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("history: prepare select error: {:?}", e);
            return;
        }
    };

    let mut rows = match stmt.query(NO_PARAMS) {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("history: query error: {:?}", e);
            return;
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
                            return;
                        }
                    };
                    let inp: String = match row.get(1) {
                        Ok(x) => x,
                        Err(e) => {
                            println_stderr!("history: error: {:?}", e);
                            return;
                        }
                    };
                    println!("{}: {}", row_id, inp);
                } else {
                    return;
                }
            }
            Err(e) => {
                println_stderr!("history: rows next error: {:?}", e);
                return;
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
