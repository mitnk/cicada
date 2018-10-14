use std::io::Write;
use std::path::Path;

use sqlite;
use sqlite::State;

use history;
use types;

pub fn run(cmd: &types::Command) -> i32 {
    let tokens = &cmd.tokens;

    let hfile = history::get_history_file();
    let path = Path::new(hfile.as_str());
    if !path.exists() {
        println_stderr!("no history file.");
        return 1;
    }

    if let Ok(conn) = sqlite::open(hfile.clone()) {
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
    } else {
        println_stderr!("history: history file open error.");
        return 1;
    }
    0
}

fn list_current_history(conn: &sqlite::Connection) -> i32 {
    let history_table = history::get_history_table();
    let q = format!(
        "SELECT rowid, inp FROM {} ORDER BY tsb desc limit 20;",
        history_table
    );
    match conn.prepare(q) {
        Ok(mut statement) => {
            let mut vec = Vec::new();
            loop {
                match statement.next() {
                    Ok(x) => {
                        if let State::Row = x {
                            let rowid = if let Ok(x) = statement.read::<String>(0) {
                                x
                            } else {
                                String::new()
                            };
                            let inp = if let Ok(x) = statement.read::<String>(1) {
                                x
                            } else {
                                String::new()
                            };
                            vec.push((rowid, inp));
                        } else {
                            break;
                        }
                    }
                    Err(e) => {
                        println_stderr!("history: statement.next error: {:?}", e);
                        return 1;
                    }
                }
            }

            for elem in vec.iter().rev() {
                println!("{}: {}", elem.0, elem.1);
            }
        }
        Err(e) => {
            println_stderr!("history: prepare error - {:?}", e);
            return 1;
        }
    }
    0
}

fn search_history(conn: &sqlite::Connection, q: &str) {
    let history_table = history::get_history_table();
    let q = format!(
        "SELECT ROWID, inp FROM {}
                     WHERE inp like '%{}%'
                     ORDER BY tsb desc limit 20;",
        history_table, q
    );
    match conn.prepare(q) {
        Ok(mut statement) => {
            let mut vec = Vec::new();
            loop {
                match statement.next() {
                    Ok(x) => {
                        if let State::Row = x {
                            let rowid = if let Ok(x) = statement.read::<String>(0) {
                                x
                            } else {
                                String::new()
                            };
                            let inp = if let Ok(x) = statement.read::<String>(1) {
                                x
                            } else {
                                String::new()
                            };
                            vec.push((rowid, inp));
                        } else {
                            break;
                        }
                    }
                    Err(e) => {
                        println_stderr!("history: statement.next error: {:?}", e);
                        return;
                    }
                }
            }
            for elem in vec.iter().rev() {
                println!("{}: {}", elem.0, elem.1);
            }
        }
        Err(e) => {
            println_stderr!("history: prepare error - {:?}", e);
            return;
        }
    }
}

fn delete_history_item(conn: &sqlite::Connection, rowid: usize) {
    let history_table = history::get_history_table();
    let sql = format!("DELETE from {} where rowid = {}", history_table, rowid);
    match conn.execute(sql) {
        Ok(_) => {
            println!("history item was deleted.");
        }
        Err(e) => {
            println_stderr!("history: prepare error - {:?}", e);
        }
    }
}
