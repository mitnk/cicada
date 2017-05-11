use std::env;
use std::fs;
use std::path::Path;

use linefeed::Reader;
use linefeed::terminal::DefaultTerminal;
use sqlite;

use tools;
use shell;

pub fn init(rl: &mut Reader<DefaultTerminal>) {
    rl.set_history_size(9999);  // make it bigger but not huge
    let hfile = get_history_file();
    let path = Path::new(hfile.as_str());
    if !path.exists() {
        let _parent = path.parent().expect("no parent found");
        let parent = _parent.to_str().expect("parent to_str error");
        fs::create_dir_all(parent).expect("dirs create failed");
        fs::File::create(hfile.as_str()).expect("file create failed");
    }
    match sqlite::open(hfile.clone()) {
        Ok(conn) => {
            match conn.execute("
                CREATE TABLE IF NOT EXISTS xonsh_history
                    (inp TEXT,
                     rtn INTEGER,
                     tsb REAL,
                     tse REAL,
                     sessionid TEXT,
                     out TEXT,
                     info TEXT
                    );
            ") {
                Ok(_) => {}
                Err(e) => tools::println_stderr(
                    format!("cicada: sqlite exec error - {:?}", e).as_str())
            }
            match conn.iterate("SELECT inp FROM xonsh_history ORDER BY tsb;",
                         |pairs| {
                    for &(_, value) in pairs.iter() {
                        let inp = value.expect("cicada: sqlite pairs error");
                        rl.add_history(inp.trim().to_string());
                    }
                    true
            }) {
                Ok(_) => {}
                Err(e) => tools::println_stderr(
                    format!("cicada: sqlite select error - {:?}", e).as_str())
            }
        }
        Err(e) => {
            tools::println_stderr(
                format!("cicada: sqlite conn error - {:?}", e).as_str());
        }
    }
}

pub fn get_history_file() -> String {
    if let Ok(hfile) = env::var("HISTORY_FILE") {
        return hfile;
    } else {
        if let Ok(d) = env::var("XDG_DATA_HOME") {
            return format!("{}/{}", d, "cicada/history.sqlite");
        } else {
            let home = tools::get_user_home();
            return format!("{}/{}", home, ".local/share/cicada/history.sqlite");
        }
    }
}

pub fn add(sh: &mut shell::Shell, rl: &mut Reader<DefaultTerminal>, line: &str, status: i32, tsb: f64, tse: f64) {
    rl.add_history(line.to_string());
    if line == sh.previous_cmd {
        return;
    }

    sh.previous_cmd = line.to_string();
    let hfile = get_history_file();
    let conn = sqlite::open(hfile).expect("sqlite open error");
    let sql = format!("INSERT INTO \
        xonsh_history (inp, rtn, tsb, tse, sessionid) \
        VALUES('{}', {}, {}, {}, '{}');",
        str::replace(line.trim(), "'", "''"),
        status, tsb, tse, "cicada");
    match conn.execute(sql) {
        Ok(_) => {}
        Err(e) => println!("failed to save history: {:?}", e)
    }
}
