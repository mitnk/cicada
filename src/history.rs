use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use linefeed::terminal::DefaultTerminal;
use linefeed::Interface;
use sqlite;

use shell;
use tools;

pub fn init(rl: &mut Interface<DefaultTerminal>) {
    let mut hist_size: usize = 999;
    if let Ok(x) = env::var("HISTORY_SIZE") {
        if let Ok(y) = x.parse::<usize>() {
            hist_size = y;
        }
    }
    rl.set_history_size(hist_size);

    let history_table = get_history_table();
    let hfile = get_history_file();
    let path = Path::new(hfile.as_str());
    if !path.exists() {
        let _parent;
        match path.parent() {
            Some(x) => _parent = x,
            None => {
                println!("cicada: history init - no parent found");
                return;
            }
        }
        let parent;
        match _parent.to_str() {
            Some(x) => parent = x,
            None => {
                println!("cicada: parent to_str is None");
                return;
            }
        }
        match fs::create_dir_all(parent) {
            Ok(_) => {}
            Err(e) => {
                println!("dirs create failed: {:?}", e);
                return;
            }
        }
        match fs::File::create(hfile.as_str()) {
            Ok(_) => {}
            Err(e) => {
                println!("file create failed: {:?}", e);
            }
        }
    }

    let mut histories: HashMap<String, bool> = HashMap::new();
    match sqlite::open(hfile.clone()) {
        Ok(conn) => {
            let sql_create = format!(
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
                history_table
            );
            match conn.execute(sql_create) {
                Ok(_) => {}
                Err(e) => println_stderr!("cicada: sqlite exec error - {:?}", e),
            }

            if let Ok(x) = env::var("HISTORY_DELETE_DUPS") {
                if x == "1" {
                    delete_duplicated_histories();
                }
            }

            let sql_select = format!("SELECT inp FROM {} ORDER BY tsb;", history_table,);
            match conn.iterate(sql_select, |pairs| {
                for &(_, value) in pairs.iter() {
                    let inp;
                    match value {
                        Some(x) => inp = x,
                        None => {
                            println!("cicada: sqlite pairs None");
                            continue;
                        }
                    }
                    let _k = inp.to_string();
                    if histories.contains_key(&_k) {
                        continue;
                    }
                    histories.insert(_k, true);
                    rl.add_history(inp.trim().to_string());
                }
                true
            }) {
                Ok(_) => {}
                Err(e) => println_stderr!("cicada: sqlite select error - {:?}", e),
            }
        }
        Err(e) => {
            println_stderr!("cicada: sqlite conn error - {:?}", e);
        }
    }
}

pub fn get_history_file() -> String {
    if let Ok(hfile) = env::var("HISTORY_FILE") {
        return hfile;
    } else if let Ok(d) = env::var("XDG_DATA_HOME") {
        return format!("{}/{}", d, "cicada/history.sqlite");
    } else {
        let home = tools::get_user_home();
        return format!("{}/{}", home, ".local/share/cicada/history.sqlite");
    }
}

pub fn get_history_table() -> String {
    if let Ok(hfile) = env::var("HISTORY_TABLE") {
        return hfile;
    } else {
        return String::from("cicada_history");
    }
}

fn delete_duplicated_histories() {
    let hfile = get_history_file();
    let history_table = get_history_table();
    match sqlite::open(hfile.clone()) {
        Ok(conn) => {
            let sql = format!(
                "DELETE FROM {} WHERE rowid NOT IN (
                SELECT MAX(rowid) FROM {} GROUP BY inp)",
                history_table, history_table
            );
            match conn.execute(sql) {
                Ok(_) => {}
                Err(e) => println_stderr!("cicada: sqlite exec error - {:?}", e),
            }
        }
        Err(e) => println_stderr!("cicada: sqlite open file error - {:?}", e),
    }
}

pub fn add(
    sh: &mut shell::Shell,
    rl: &mut Interface<DefaultTerminal>,
    line: &str,
    status: i32,
    tsb: f64,
    tse: f64,
) {
    if sh.cmd.starts_with(' ') {
        return;
    }

    sh.previous_status = status;
    if line == sh.previous_cmd {
        return;
    }
    rl.add_history(line.to_string());

    sh.previous_cmd = line.to_string();
    let hfile = get_history_file();
    let history_table = get_history_table();
    let conn;
    match sqlite::open(hfile) {
        Ok(x) => conn = x,
        Err(e) => {
            println!("cicada: sqlite open db error: {:?}", e);
            return;
        }
    }
    let sql = format!(
        "INSERT INTO \
         {} (inp, rtn, tsb, tse, sessionid) \
         VALUES('{}', {}, {}, {}, '{}');",
        history_table,
        str::replace(line.trim(), "'", "''"),
        status,
        tsb,
        tse,
        "cicada"
    );
    match conn.execute(sql) {
        Ok(_) => {}
        Err(e) => println!("failed to save history: {:?}", e),
    }
}
