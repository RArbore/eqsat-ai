use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU16;

pub(crate) type Symbol = SymbolU16;
pub(crate) type Interner = StringInterner<StringBackend<Symbol>>;
