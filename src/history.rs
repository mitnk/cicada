use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use linefeed::terminal::DefaultTerminal;
use linefeed::Interface;
use rusqlite::Connection as Conn;
use rusqlite::Error::SqliteFailure;

use crate::shell;
use crate::tools;

fn init_db(hfile: &str, htable: &str) {
    let path = Path::new(hfile);
    if !path.exists() {
        let _parent = match path.parent() {
            Some(x) => x,
            None => {
                println_stderr!("cicada: history init - no parent found");
                return;
            }
        };
        let parent = match _parent.to_str() {
            Some(x) => x,
            None => {
                println_stderr!("cicada: parent to_str is None");
                return;
            }
        };
        match fs::create_dir_all(parent) {
            Ok(_) => {}
            Err(e) => {
                println_stderr!("cicada: histdir create error: {}", e);
                return;
            }
        }
        match fs::File::create(hfile) {
            Ok(_) => {
                println!("cicada: created history file: {}", hfile);
            }
            Err(e) => {
                println_stderr!("cicada: history: file create failed: {}", e);
            }
        }
    }

    let conn = match Conn::open(hfile) {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("cicada: history: open db error: {}", e);
            return;
        }
    };
    let sql = format!(
        "
        CREATE TABLE IF NOT EXISTS {}
            (inp TEXT,
             rtn INTEGER,
             tsb REAL,
             tse REAL,
             sessionid TEXT,
             out TEXT,
             info TEXT
            );
    ",
        htable
    );
    match conn.execute(&sql, []) {
        Ok(_) => {}
        Err(e) => println_stderr!("cicada: history: query error: {}", e),
    }
}

pub fn init(rl: &mut Interface<DefaultTerminal>) {
    let mut hist_size: usize = 99999;
    if let Ok(x) = env::var("HISTORY_SIZE") {
        if let Ok(y) = x.parse::<usize>() {
            hist_size = y;
        }
    }
    rl.set_history_size(hist_size);

    let history_table = get_history_table();
    let hfile = get_history_file();

    if !Path::new(&hfile).exists() {
        init_db(&hfile, &history_table);
    }

    let mut delete_dups = true;
    if let Ok(x) = env::var("HISTORY_DELETE_DUPS") {
        if x == "0" {
            delete_dups = false;
        }
    }
    if delete_dups {
        delete_duplicated_histories();
    }

    let conn = match Conn::open(&hfile) {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("cicada: history: conn error: {}", e);
            return;
        }
    };
    let sql = format!("SELECT inp FROM {} ORDER BY tsb;", history_table);
    let mut stmt = match conn.prepare(&sql) {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("cicada: prepare select error: {}", e);
            return;
        }
    };

    let rows = match stmt.query_map([], |row| row.get(0)) {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("cicada: query select error: {}", e);
            return;
        }
    };

    let mut dict_helper: HashMap<String, bool> = HashMap::new();
    for x in rows.flatten() {
        let inp: String = x;
        if dict_helper.contains_key(&inp) {
            continue;
        }
        dict_helper.insert(inp.clone(), true);
        rl.add_history(inp.trim().to_string());
    }
}

pub fn get_history_file() -> String {
    if let Ok(hfile) = env::var("HISTORY_FILE") {
        hfile
    } else if let Ok(d) = env::var("XDG_DATA_HOME") {
        format!("{}/{}", d, "cicada/history.sqlite")
    } else {
        let home = tools::get_user_home();
        format!("{}/{}", home, ".local/share/cicada/history.sqlite")
    }
}

pub fn get_history_table() -> String {
    if let Ok(hfile) = env::var("HISTORY_TABLE") {
        hfile
    } else {
        String::from("cicada_history")
    }
}

fn delete_duplicated_histories() {
    let hfile = get_history_file();
    let history_table = get_history_table();
    let conn = match Conn::open(&hfile) {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("cicada: history: conn error: {}", e);
            return;
        }
    };
    let sql = format!(
        "DELETE FROM {} WHERE rowid NOT IN (
        SELECT MAX(rowid) FROM {} GROUP BY inp)",
        history_table, history_table
    );
    match conn.execute(&sql, []) {
        Ok(_) => {}
        Err(e) => match e {
            SqliteFailure(ee, msg) => {
                if ee.extended_code == 5 {
                    log!(
                        "failed to delete dup histories: {}",
                        msg.unwrap_or("db is locked?".to_owned()),
                    );
                    return;
                }
                println_stderr!(
                    "cicada: history: delete dups error: {}: {:?}",
                    &ee,
                    &msg
                );
            }
            _ => {
                println_stderr!("cicada: history: delete dup error: {}", e);
            }
        },
    }
}

pub fn add_raw(sh: &shell::Shell, line: &str, status: i32,
               tsb: f64, tse: f64) {
    let hfile = get_history_file();
    let history_table = get_history_table();
    if !Path::new(&hfile).exists() {
        init_db(&hfile, &history_table);
    }

    let conn = match Conn::open(&hfile) {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("cicada: history: conn error: {}", e);
            return;
        }
    };
    let sql = format!(
        "INSERT INTO \
         {} (inp, rtn, tsb, tse, sessionid, info) \
         VALUES('{}', {}, {}, {}, '{}', 'dir:{}|');",
        history_table,
        str::replace(line.trim(), "'", "''"),
        status,
        tsb,
        tse,
        sh.session_id,
        sh.current_dir,
    );
    match conn.execute(&sql, []) {
        Ok(_) => {}
        Err(e) => println_stderr!("cicada: history: save error: {}", e),
    }
}

pub fn add(sh: &shell::Shell, rl: &mut Interface<DefaultTerminal>, line: &str,
           status: i32, tsb: f64, tse: f64) {
    add_raw(sh, line, status, tsb, tse);
    rl.add_history(line.to_string());
}
