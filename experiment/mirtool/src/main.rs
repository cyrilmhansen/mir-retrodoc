mod commands;
mod error;
mod io;


struct Args {
    command: String,
    input: String,
    output: Option<String>,
    format: Option<String>,
    entry: Option<String>,
    trace: bool,
    force: bool,
    keep_temp: bool,
}

fn show_help() -> String {
    let mut s = String::new();
    s.push_str("mirtool: Developer CLI for the MIR-F0 experimental pipeline\n\n");
    s.push_str("Usage:\n");
    s.push_str("  mirtool <command> [arguments] [options]\n\n");
    s.push_str("Commands:\n");
    s.push_str("  validate <input_file>                     Loads and statically validates a module image.\n");
    s.push_str("  encode <input_file> <output_file>          Encodes textual mircap to Cap'n Proto binary.\n");
    s.push_str("  decode <input_file>                       Decodes binary mircap to a readable debug layout.\n");
    s.push_str("  run <input_file>                          Executes entry function with mirsem reference interpreter.\n");
    s.push_str("  compile-c <input_file> <output_file>      Transpiles a module image to portable C11.\n");
    s.push_str("  diff <input_file>                         Runs differential execution comparison between mirsem and compiled C.\n\n");
    s.push_str("Options:\n");
    s.push_str("  --format <text|binary>                    Explicitly specify input file format.\n");
    s.push_str("  --entry <name>                            Set entry function name (default: main).\n");
    s.push_str("  --trace                                   Show trace snapshot summary after running.\n");
    s.push_str("  --force                                   Overwrite encode output file if it already exists.\n");
    s.push_str("  --keep-temp                               Retain temporary files during differential verification.\n");
    s
}

fn parse_args() -> Result<Args, String> {
    let mut args_iter = std::env::args().skip(1);
    let command = args_iter.next().ok_or_else(|| show_help())?;
    
    if command == "help" || command == "-h" || command == "--help" {
        return Err(show_help());
    }

    let mut format = None;
    let mut entry = None;
    let mut trace = false;
    let mut force = false;
    let mut keep_temp = false;

    let mut positional = Vec::new();

    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--trace" => trace = true,
            "--force" => force = true,
            "--keep-temp" => keep_temp = true,
            "--format" => {
                let val = args_iter.next().ok_or("Expected value after --format option")?;
                if val != "text" && val != "binary" {
                    return Err("Format must be either 'text' or 'binary'".to_string());
                }
                format = Some(val);
            }
            "--entry" => {
                let val = args_iter.next().ok_or("Expected value after --entry option")?;
                entry = Some(val);
            }
            _ if arg.starts_with("-") => {
                return Err(format!("Unknown option: {}\n\n{}", arg, show_help()));
            }
            _ => {
                positional.push(arg);
            }
        }
    }

    let (input, output) = match command.as_str() {
        "validate" | "decode" | "run" | "diff" => {
            if positional.is_empty() {
                return Err(format!("Command '{}' requires an input file path.\n\n{}", command, show_help()));
            }
            if positional.len() > 1 {
                return Err(format!("Command '{}' does not accept additional positional arguments: {:?}.\n\n{}", command, &positional[1..], show_help()));
            }
            (positional[0].clone(), None)
        }
        "encode" | "compile-c" => {
            if positional.len() < 2 {
                return Err(format!("Command '{}' requires both input and output file paths.\n\n{}", command, show_help()));
            }
            if positional.len() > 2 {
                return Err(format!("Command '{}' does not accept additional positional arguments: {:?}.\n\n{}", command, &positional[2..], show_help()));
            }
            (positional[0].clone(), Some(positional[1].clone()))
        }
        _ => {
            return Err(format!("Unknown command: '{}'.\n\n{}", command, show_help()));
        }
    };

    Ok(Args {
        command,
        input,
        output,
        format,
        entry,
        trace,
        force,
        keep_temp,
    })
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(help) => {
            eprintln!("{}", help);
            std::process::exit(1);
        }
    };

    let entry_name = args.entry.as_deref().unwrap_or("main");

    let result = match args.command.as_str() {
        "validate" => commands::cmd_validate(&args.input, args.format.as_deref()),
        "encode" => commands::cmd_encode(&args.input, args.output.as_ref().unwrap(), args.force),
        "decode" => commands::cmd_decode(&args.input, args.format.as_deref()),
        "run" => commands::cmd_run(&args.input, args.format.as_deref(), entry_name, args.trace),
        "compile-c" => commands::cmd_compile_c(&args.input, args.output.as_ref().unwrap(), args.format.as_deref(), entry_name),
        "diff" => commands::cmd_diff(&args.input, args.format.as_deref(), entry_name, args.keep_temp),
        _ => unreachable!(),
    };

    if let Err(err) = result {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
