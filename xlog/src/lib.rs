use lalrpop_util::lalrpop_mod;

pub mod database;
pub mod frontend;
pub mod query;

lalrpop_mod!(pub grammar);
