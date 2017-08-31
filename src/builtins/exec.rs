use exec;
use parsers;

pub fn run(tokens: &Vec<(String, String)>) -> i32 {
    let args = parsers::parser_line::tokens_to_args(&tokens);
    let len = args.len();
    if len == 1 {
        println!("invalid command");
        return 1;
    }

    let mut cmd = exec::Command::new(&args[1]);
    let err = cmd.args(&args[2..len]).exec();
    println!("exec error: {:?}", err);
    0
}
