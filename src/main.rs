extern crate ansi_term;
extern crate rustyline;

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
    loop {
        let prompt;
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
                let args : Vec<&str> = line.trim().split(' ').collect();
                match Command::new(args[0]).args(&(args[1..])).output() {
                    Ok(output) => {
                        proc_status_ok = output.status.success();
                        let err = String::from_utf8_lossy(&output.stderr);
                        if err != "" {
                            print!("{}", err);
                        }
                        let out = String::from_utf8_lossy(&output.stdout);
                        if out != "" {
                            print!("{}", out);
                        }
                    },
                    Err(e) => {
                        proc_status_ok = false;
                        println!("{:?}", e);
                    }
                }
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
