mod parser;
mod compiler;
mod theory;
mod tui;

use clap::Parser;
use std::fs;
use std::path::PathBuf;
use crate::parser::parse_tabbit;
use crate::compiler::Compiler;
use crate::tui::TuiApp;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The .tabbit file to load
    input: PathBuf,

    /// Dump the compiled song data and exit
    #[arg(short, long)]
    dump: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    let content = fs::read_to_string(&args.input)?;
    let commands = parse_tabbit(&content);
    
    let mut compiler = Compiler::new();
    let song = compiler.compile(&commands);
    
    if args.dump {
        println!("{:#?}", song);
        return Ok(());
    }
    
    let mut app = TuiApp::new(song);
    app.run()?;
    
    Ok(())
}
