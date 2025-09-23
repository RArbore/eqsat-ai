use std::io::{Read, Result, stdin};

use ds::uf::UnionFind;

use xlog::database::{Database, DatabaseAuxiliaryState};
use xlog::fixpoint::fixpoint;
use xlog::frontend::Interner;

use imp::ai::abstract_interpret;
use imp::grammar::ProgramParser;

pub fn main() -> Result<()> {
    let uf = UnionFind::new();
    let mut interner = Interner::new();
    let aux_state = DatabaseAuxiliaryState { uf: &uf };
    let mut database = Database::new(aux_state);

    let mut imp_program = String::new();
    stdin().read_to_string(&mut imp_program)?;
    let mut location = 0;
    let ast = ProgramParser::new().parse(&mut interner, &mut location, &imp_program).unwrap();

    let rules = abstract_interpret(&ast, &mut database, &mut interner);
    fixpoint(&mut database, &rules);
    database.dump(&interner);

    Ok(())
}
