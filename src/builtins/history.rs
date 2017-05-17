use std::io::Write;
use std::path::Path;

use sqlite;
use sqlite::State;

use history;

pub fn run(args: Vec<String>) -> i32 {
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
    let q = format!("SELECT inp FROM {} ORDER BY tsb desc limit 10;",
                    history_table);
    match conn.prepare(q) {
        Ok(mut statement) => {
            let mut vec = Vec::new();
            while let State::Row = statement.next().expect("history: statement next error") {
                if let Ok(x) = statement.read::<String>(0) {
                    vec.push(x);
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
    let q = format!("SELECT inp FROM {}
                     WHERE inp like '%{}%'
                     ORDER BY tsb desc limit 20;",
                    history_table,
                    q);
    match conn.prepare(q) {
        Ok(mut statement) => {
            let mut vec = Vec::new();
            while let State::Row = statement.next().expect("history: statement next error") {
                if let Ok(x) = statement.read::<String>(0) {
                    vec.push(x);
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
