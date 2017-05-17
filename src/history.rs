use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use linefeed::Reader;
use linefeed::terminal::DefaultTerminal;
use sqlite;

use tools;
use shell;

pub fn init(rl: &mut Reader<DefaultTerminal>) {
    let mut hist_size: usize = 9999; // make default bigger but not huge
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
        let _parent = path.parent().expect("no parent found");
        let parent = _parent.to_str().expect("parent to_str error");
        fs::create_dir_all(parent).expect("dirs create failed");
        fs::File::create(hfile.as_str()).expect("file create failed");
    }

    let mut histories: HashMap<String, bool> = HashMap::new();
    match sqlite::open(hfile.clone()) {
        Ok(conn) => {
            let sql_create = format!("
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
                                     history_table);
            match conn.execute(sql_create) {
                Ok(_) => {}
                Err(e) => println_stderr!("cicada: sqlite exec error - {:?}", e),
            }
            let sql_select = format!(
                "SELECT inp FROM {} ORDER BY tsb;",
                history_table,
            );
            match conn.iterate(sql_select, |pairs| {
                for &(_, value) in pairs.iter() {
                    let inp = value.expect("cicada: sqlite pairs error");
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

pub fn add(sh: &mut shell::Shell,
           rl: &mut Reader<DefaultTerminal>,
           line: &str,
           status: i32,
           tsb: f64,
           tse: f64) {
    rl.add_history(line.to_string());
    if line == sh.previous_cmd {
        return;
    }

    sh.previous_cmd = line.to_string();
    let hfile = get_history_file();
    let history_table = get_history_table();
    let conn = sqlite::open(hfile).expect("sqlite open error");
    let sql = format!("INSERT INTO \
        {} (inp, rtn, tsb, tse, sessionid) \
        VALUES('{}', {}, {}, {}, '{}');",
                      history_table,
                      str::replace(line.trim(), "'", "''"),
                      status,
                      tsb,
                      tse,
                      "cicada");
    match conn.execute(sql) {
        Ok(_) => {}
        Err(e) => println!("failed to save history: {:?}", e),
    }
}
