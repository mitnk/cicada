use std::env;
use std::fs;
use std::path::Path;

use linefeed::Reader;
use linefeed::terminal::DefaultTerminal;
use sqlite;

use tools;

pub fn init(rl: &mut Reader<DefaultTerminal>) {
    rl.set_history_size(9999);  // make it bigger but not huge
    let hfile = get_history_file();
    let path = Path::new(hfile.as_str());
    if !path.exists() {
        let parent = path.parent().expect("no parent found");
        fs::create_dir_all(parent.to_str().unwrap()).expect("dirs create failed");
        fs::File::create(hfile.as_str()).expect("file create failed");
    }

    let conn = sqlite::open(hfile.clone()).unwrap();
    conn.execute("
        CREATE TABLE IF NOT EXISTS xonsh_history
            (inp TEXT,
             rtn INTEGER,
             tsb REAL,
             tse REAL,
             sessionid TEXT,
             out TEXT,
             info TEXT
            );
    ").unwrap();
    conn.iterate("SELECT DISTINCT inp FROM xonsh_history ORDER BY tsb;",
                 |pairs| {
            for &(_, value) in pairs.iter() {
                let inp = value.unwrap();
                rl.add_history(inp.trim().to_string());
            }
            true
        })
        .unwrap();
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

pub fn add(rl: &mut Reader<DefaultTerminal>, line: &str, status: i32, tss: f64, tse: f64) {
    rl.add_history(line.to_string());
    let hfile = get_history_file();
    let conn = sqlite::open(hfile).expect("sqlite open error");
    let sql = format!("INSERT INTO \
        xonsh_history (inp, rtn, tsb, tse, sessionid) \
        VALUES('{}', {}, {}, {}, '{}');",
        str::replace(line, "'", "''"),
        status, tss, tse, "cicada");
    match conn.execute(sql) {
        Ok(_) => {}
        Err(e) => println!("failed to save history: {:?}", e)
    }
}
