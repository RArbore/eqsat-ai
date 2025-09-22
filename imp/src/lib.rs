use lalrpop_util::lalrpop_mod;

pub mod ai;
pub mod ast;
pub mod lattice;

lalrpop_mod!(pub grammar);
