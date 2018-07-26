use std::io::Write;
use std::path::Path;

use sqlite;
use sqlite::State;

use history;
use parsers;

pub fn run(tokens: &Vec<(String, String)>) -> i32 {
    let args = parsers::parser_line::tokens_to_args(&tokens);
    let hfile = history::get_history_file();
    let path = Path::new(hfile.as_str());
    if !path.exists() {
        println_stderr!("no history file.");
        return 1;
    }

    if let Ok(conn) = sqlite::open(hfile.clone()) {
        if args.len() == 1 {
            return list_current_history(&conn);
        } else if args.len() == 2 {
            search_history(&conn, args[1].as_str());
        } else {
            println_stderr!("history: only take one arg");
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
        "SELECT inp FROM {} ORDER BY tsb desc limit 20;",
        history_table
    );
    match conn.prepare(q) {
        Ok(mut statement) => {
            let mut vec = Vec::new();
            loop {
                match statement.next() {
                    Ok(x) => {
                        if let State::Row = x {
                            if let Ok(_x) = statement.read::<String>(0) {
                                vec.push(_x);
                            }
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

            for (i, elem) in vec.iter().rev().enumerate() {
                println!("{}: {}", i, elem);
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
        "SELECT inp FROM {}
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
                            if let Ok(_x) = statement.read::<String>(0) {
                                vec.push(_x);
                            }
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
            for (i, elem) in vec.iter().rev().enumerate() {
                println!("{}: {}", i, elem);
            }
        }
        Err(e) => {
            println_stderr!("history: prepare error - {:?}", e);
        }
    }
}
