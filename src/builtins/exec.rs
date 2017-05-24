use exec;

#[allow(needless_pass_by_value)]
pub fn run(args: Vec<String>) -> i32 {
    let len = args.len();
    if len == 1 {
        println!("invalid command");
        return 1;
    }

    let mut cmd = exec::Command::new(&args[1]);
    let err = cmd.args(&args[2..len]).exec();
    println!("exec error: {:?}", err);
    return 0;
}
