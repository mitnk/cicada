extern crate ansi_term;
extern crate errno;
extern crate libc;
extern crate linefeed;
extern crate nix;
extern crate regex;
extern crate shlex;
extern crate sqlite;
extern crate time;
extern crate yaml_rust;

#[macro_use]
extern crate nom;

use std::env;
use std::io;
use std::path::Path;
use std::rc::Rc;

// use std::thread;
// use std::time::Duration;

use ansi_term::Colour::Red;
use ansi_term::Colour::Green;

use nom::IResult;
use regex::Regex;

use linefeed::{Reader, ReadResult};
use linefeed::{Command, Function, Terminal};

mod builtins;
mod completers;
mod execute;
mod jobs;
mod parsers;
mod tools;

struct DemoFunction;
const DEMO_FN_SEQ: &'static str = "\x1b[A"; // Ctrl-X, d

impl<Term: Terminal> Function<Term> for DemoFunction {
    fn execute(&self, reader: &mut Reader<Term>, _count: i32, _ch: char) -> io::Result<()> {
        assert_eq!(reader.sequence(), DEMO_FN_SEQ);
        reader.insert_str("<todo>")
    }
}




fn main() {
    if env::args().len() > 1 {
        println!("does not support args yet.");
        return;
    }

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    let user = env::var("USER").unwrap();
    let home = tools::get_user_home();
    let env_path = env::var("PATH").unwrap();
    let dir_bin_cargo = format!("{}/.cargo/bin", home);
    let env_path_new = ["/usr/local/bin".to_string(),
                        env_path,
                        dir_bin_cargo,
                        "/Library/Frameworks/Python.framework/Versions/3.6/bin".to_string(),
                        "/Library/Frameworks/Python.framework/Versions/3.5/bin".to_string(),
                        "/Library/Frameworks/Python.framework/Versions/3.4/bin".to_string(),
                        "/Library/Frameworks/Python.framework/Versions/2.7/bin".to_string()]
            .join(":");
    env::set_var("PATH", &env_path_new);

    let mut previous_dir = String::new();
    let mut proc_status_ok = true;
    let mut painter;

    let mut rl = Reader::new("demo").unwrap();
    // rl.read_init();
    rl.set_completer(Rc::new(completers::DemoCompleter));

    rl.define_function("demo-function", Rc::new(DemoFunction));
    rl.bind_sequence(DEMO_FN_SEQ, Command::from_str("demo-function"));

    let file_db = format!("{}/{}", home, ".local/share/xonsh/xonsh-history.sqlite");
    if Path::new(file_db.as_str()).exists() {
        let conn = sqlite::open(file_db).unwrap();
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
                    rl.add_history(inp.to_string());
                }
                true
            })
            .unwrap();
    }

    loop {
        if proc_status_ok {
            painter = Green;
        } else {
            painter = Red;
        }

        let _current_dir = env::current_dir().unwrap();
        let current_dir = _current_dir.to_str().unwrap();
        let _tokens: Vec<&str> = current_dir.split("/").collect();

        let last = _tokens.last().unwrap();
        let pwd: String;
        if last.to_string() == "" {
            pwd = String::from("/");
        } else if current_dir == home {
            pwd = String::from("~");
        } else {
            pwd = last.to_string();
        }
        let prompt = format!("{}@{}: {}$ ",
                             painter.paint(user.to_string()),
                             painter.paint("MT"),
                             painter.paint(pwd));
        rl.set_prompt(prompt.as_str());
        if let Ok(ReadResult::Input(line)) = rl.read_line() {
            let cmd: String;
            if line.trim() == "exit" {
                break;
            } else if line.trim() == "" {
                continue;
            } else if line.trim() == "version" {
                println!("MT shell v{} by @mitnk", VERSION);
                continue;
            } else if line.trim() == "bash" {
                cmd = String::from("bash --rcfile ~/.bash_profile");
            } else {
                cmd = line.to_string();
            }

            let time_started = time::get_time();
            rl.add_history(cmd.to_string());
            let file_db = format!("{}/{}", home, ".local/share/xonsh/xonsh-history.sqlite");
            if Path::new(file_db.as_str()).exists() {
                let conn = sqlite::open(file_db).unwrap();
                let sql = format!("INSERT INTO \
                    xonsh_history (inp, rtn, tsb, tse, sessionid) \
                    VALUES('{}', {}, {}, {}, '{}');",
                    str::replace(cmd.as_str(), "'", "''"),
                    0, time_started.sec, time_started.sec as f64 + 0.01, "cicada");
                match conn.execute(sql) {
                    Ok(_) => {}
                    Err(e) => println!("failed to save history: {:?}", e)
                }
            }

            let re;
            if let Ok(x) = Regex::new(r"^[ 0-9\.\(\)\+\-\*/]+$") {
                re = x;
            } else {
                println!("regex error for arithmetic");
                continue;
            }
            if re.is_match(line.as_str()) {
                if line.contains(".") {
                    match parsers::parser_float::expr_float(line.as_bytes()) {
                        IResult::Done(_, x) => {
                            println!("{:?}", x);
                        }
                        IResult::Error(x) => println!("Error: {:?}", x),
                        IResult::Incomplete(x) => println!("Incomplete: {:?}", x),
                    }
                } else {
                    match parsers::parser_int::expr_int(line.as_bytes()) {
                        IResult::Done(_, x) => {
                            println!("{:?}", x);
                        }
                        IResult::Error(x) => println!("Error: {:?}", x),
                        IResult::Incomplete(x) => println!("Incomplete: {:?}", x),
                    }
                }
                continue;
            }

            let args;
            if let Some(x) = shlex::split(cmd.trim()) {
                args = x;
            } else {
                println!("shlex split error: does not support multiple line");
                proc_status_ok = false;
                continue;
            }
            if args[0] == "cd" {
                let result = builtins::cd::run(args.clone(),
                                               home.as_str(),
                                               current_dir,
                                               &mut previous_dir);
                proc_status_ok = result == 0;
                continue;
            } else {
                let len = args.len();
                let result;
                if len > 2 && (args[len - 2] == ">" || args[len - 2] == ">>") {
                    let append = args[len - 2] == ">>";
                    let mut args_new = args.clone();
                    let redirect = args_new.pop().unwrap();
                    args_new.pop();
                    result = execute::run_pipeline(
                        args_new, redirect.as_str(), append);
                } else {
                    result = execute::run_pipeline(args.clone(), "", false);
                }
                proc_status_ok = result == 0;
                unsafe {
                    let gid = libc::getpgid(0);
                    tools::rlog(format!("try return term to {}\n", gid));
                    jobs::give_terminal_to(gid);
                }
                continue;
            }
        } else {
            println!("rl.read_line() error");
        }
    }
}
