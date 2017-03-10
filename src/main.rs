extern crate ansi_term;
extern crate rustyline;
extern crate shlex;

use std::env;
use std::process::Command;

use ansi_term::Colour::Red;
use ansi_term::Colour::Green;
use rustyline::Editor;
use rustyline::error::ReadlineError;


fn main() {
    println!("RUSH v0.1.0 Tell me what to do!");
    let mut rl = Editor::<()>::new();
    let user = env::var("USER").unwrap();
    let mut proc_status_ok = true;
    let mut prompt;
    loop {
        if proc_status_ok {
            prompt = format!("{}@{} ",
                             Green.paint(user.to_string()),
                             Green.paint("RUSH: ~$"));
        } else {
            prompt = format!("{}@{} ",
                             Red.paint(user.to_string()),
                             Red.paint("RUSH: ~$"));
        }

        let cmd = rl.readline(prompt.as_str());
        match cmd {
            Ok(line) => {
                if line.trim() == "exit" {
                    println!("Bye.");
                    break;
                } else if line.trim() == "" {
                    continue;
                }
                rl.add_history_entry(&line);

                let args = shlex::split(line.trim()).unwrap();
                let mut child;
                match Command::new(&args[0]).args(&(args[1..])).spawn() {
                    Ok(x) => child = x,
                    Err(e) => {
                        proc_status_ok = false;
                        println!("{:?}", e);
                        continue
                    }
                }
                let ecode = child.wait().expect("failed to wait");
                proc_status_ok = ecode.success();
            },
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C
                continue;
            },
            Err(ReadlineError::Eof) => {
                // Ctrl-D
                continue;
            },
            Err(err) => {
                println!("RL Error: {:?}", err);
                continue;
            }
        }
    }
}
