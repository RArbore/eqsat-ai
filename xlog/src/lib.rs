use lalrpop_util::lalrpop_mod;

pub mod action;
pub mod database;
pub mod fixpoint;
pub mod frontend;
pub mod query;

lalrpop_mod!(pub grammar);
