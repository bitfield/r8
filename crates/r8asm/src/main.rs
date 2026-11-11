use anyhow::Result;
use clap::Parser;

use std::{fs, path::PathBuf};

use r8asm::{
    asm::{Assembler, Disassembler},
    token::tokenize_source_file,
};

/// An assembler for the R8 fantasy retro CPU.
#[expect(clippy::struct_excessive_bools, reason = "I'll be the judge of that")]
#[derive(Debug, Parser)]
struct Args {
    /// Disassemble.
    #[clap(short, long, conflicts_with_all = ["list", "format", "stdoutformat"])]
    dis: bool,
    /// Format source code.
    #[clap(short, long, conflicts_with_all = ["list", "dis", "stdoutformat"])]
    format: bool,
    /// Generate listing.
    #[clap(short, long)]
    list: bool,
    /// Paths to the source or binary files.
    #[clap(required = true)]
    paths: Vec<PathBuf>,
    /// Format source code to stdout.
    #[clap(short, long, conflicts_with_all = ["list", "format", "dis"])]
    stdoutformat: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    for path in args.paths {
        if args.dis {
            let code = fs::read(&path)?;
            for source in Disassembler::from(code.as_slice()) {
                println!("    {source}");
            }
        } else {
            let tokens = tokenize_source_file(&path)?;
            let mut asm = Assembler::new(tokens);
            asm.path = path.display().to_string();
            let data = asm.assemble()?;
            if args.format {
                fs::write(path, asm.source)?;
                continue;
            }
            if args.stdoutformat {
                print!("{}", asm.source);
                continue;
            }
            if args.list {
                println!("{}", asm.listing);
            }
            let mut bin_path = path.clone();
            bin_path.set_extension("bin");
            fs::write(bin_path, data)?;
        }
    }
    Ok(())
}
