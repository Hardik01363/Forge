use forge::codegen::Codegen;
use forge::ir::IrLowerer;
use forge::parser::Parser;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

fn print_usage_and_exit() -> ! {
    eprintln!("Usage: forge <source.c> [--lex | --parse | --codegen | -S] [-o <output>]");
    exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage_and_exit();
    }

    let mut source_file: Option<String> = None;
    let mut output_file: Option<String> = None;
    let mut lex_only = false;
    let mut parse_only = false;
    let mut codegen_only = false;
    let mut emit_asm_only = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--lex" => lex_only = true,
            "--parse" => parse_only = true,
            "--codegen" => codegen_only = true,
            "-S" => emit_asm_only = true,
            "-o" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: -o requires an argument");
                    exit(1);
                }
                output_file = Some(args[i].clone());
            }
            arg if arg.starts_with('-') => {
                eprintln!("Error: unrecognized flag '{arg}'");
                print_usage_and_exit();
            }
            file => {
                if source_file.is_some() {
                    eprintln!("Error: multiple source files provided");
                    print_usage_and_exit();
                }
                source_file = Some(file.to_string());
            }
        }
        i += 1;
    }

    let source_path_str = source_file.unwrap_or_else(|| {
        eprintln!("Error: no source file provided");
        print_usage_and_exit();
    });

    let source_path = Path::new(&source_path_str);
    let preprocessed_path = source_path.with_extension("i");

    //preprocessing source with gcc -E -P
    let pp_status = Command::new("gcc")
        .arg("-E")
        .arg("-P")
        .arg(source_path)
        .arg("-o")
        .arg(&preprocessed_path)
        .status();

    match pp_status {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("Preprocessing failed with status: {status}");
            exit(status.code().unwrap_or(1));
        }
        Err(err) => {
            eprintln!("Failed to execute gcc preprocessor: {err}");
            exit(1);
        }
    }

    let source_code = fs::read_to_string(&preprocessed_path).unwrap_or_else(|err| {
        eprintln!("Error reading '{source_path_str}': {err}");
        let _ = fs::remove_file(&preprocessed_path);
        exit(1);
    });

    //removing the temporary preprocessed file
    let _ = fs::remove_file(&preprocessed_path);

    //lexing phase
    if lex_only {
        let lexer = forge::lexer::Lexer::new(&source_code);
        for token_res in lexer {
            match token_res {
                Ok(token) => println!("{token:?}"),
                Err(err) => {
                    eprintln!("LexError at {:?}: {}", err.span, err.message);
                    exit(1);
                }
            }
        }
        return;
    }

    //parsing phase
    let parsed = match Parser::parse_program(&source_code) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("ParseError at {:?}: {}", err.span, err.message);
            exit(1);
        }
    };

    if parse_only {
        return;
    }

    //IR lowering phase
    let ir = IrLowerer::lower_program(&parsed.arena, &parsed.program);

    //x86 code generation phase
    let x86 = Codegen::generate_x86(&ir);

    if codegen_only {
        return;
    }

    let asm_text = Codegen::emit_assembly(&x86);

    let source_path = Path::new(&source_path_str);
    let default_asm_path = source_path.with_extension("s");

    if emit_asm_only {
        let target_asm = output_file.map(PathBuf::from).unwrap_or(default_asm_path);
        fs::write(&target_asm, asm_text).unwrap_or_else(|err| {
            eprintln!(
                "Error writing assembly to '{}': {err}",
                target_asm.display()
            );
            exit(1);
        });
        return;
    }

    //full compilation: write assembly to a temporary .s, then invoke GCC to assemble and link
    let temp_asm_path = source_path.with_extension("s");
    fs::write(&temp_asm_path, &asm_text).unwrap_or_else(|err| {
        eprintln!("Error writing assembly: {err}");
        exit(1);
    });

    let target_binary = output_file
        .map(PathBuf::from)
        .unwrap_or_else(|| source_path.with_extension(""));

    let gcc_status = Command::new("gcc")
        .arg(&temp_asm_path)
        .arg("-o")
        .arg(&target_binary)
        .status();

    match gcc_status {
        Ok(status) if status.success() => {
            //cleanup the intermediate .s file
            let _ = fs::remove_file(&temp_asm_path);
        }
        Ok(status) => {
            eprintln!("GCC assembly/linking failed with status: {status}");
            exit(status.code().unwrap_or(1));
        }
        Err(err) => {
            eprintln!("Failed to execute gcc: {err}");
            exit(1);
        }
    }
}
