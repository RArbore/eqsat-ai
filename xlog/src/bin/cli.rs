use std::io::{Result, stdin};

use ds::uf::UnionFind;

use xlog::database::{Database, DatabaseAuxiliaryState};
use xlog::fixpoint::{FunctionLibrary, fixpoint};
use xlog::frontend::Interner;
use xlog::grammar::ProgramParser;

pub fn main() -> Result<()> {
    let uf = UnionFind::new();
    let mut interner = Interner::new();
    let aux_state = DatabaseAuxiliaryState { uf: &uf };
    let mut database = Database::new(aux_state);
    let library = FunctionLibrary::new();
    let mut program = vec![];
    for line in stdin().lines() {
        let mut line = line?;
        if !line.ends_with(";") {
            line += ";";
        }
        let line = ProgramParser::new().parse(&mut interner, &mut database, &library, &line);
        match line {
            Ok(rules) => program.extend(rules),
            Err(err) => println!("{}", err),
        }
    }
    fixpoint(&mut database, &program);
    println!("{:?}", database);
    Ok(())
}
