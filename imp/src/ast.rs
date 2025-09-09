use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU16;

pub type Symbol = SymbolU16;
pub type Interner = StringInterner<StringBackend<Symbol>>;

#[derive(Debug)]
pub struct ProgramAST {
    pub funcs: Vec<FunctionAST>,
}

#[derive(Debug)]
pub struct FunctionAST {
    pub name: Symbol,
    pub params: Vec<Symbol>,
    pub block: BlockAST,
}

#[derive(Debug)]
pub struct BlockAST {
    pub stmts: Vec<StatementAST>,
}

#[derive(Debug)]
pub enum StatementAST {
    Block(BlockAST),
    Assign(Symbol, ExpressionAST),
    IfElse(ExpressionAST, BlockAST, Option<BlockAST>),
    While(ExpressionAST, BlockAST),
    Return(ExpressionAST),
}

#[derive(Debug)]
pub enum ExpressionAST {
    NumberLiteral(i32),
    Variable(Symbol),

    Call(Symbol, Vec<ExpressionAST>),

    Add(Box<ExpressionAST>, Box<ExpressionAST>),
    Subtract(Box<ExpressionAST>, Box<ExpressionAST>),
    Multiply(Box<ExpressionAST>, Box<ExpressionAST>),
    Divide(Box<ExpressionAST>, Box<ExpressionAST>),
    Modulo(Box<ExpressionAST>, Box<ExpressionAST>),

    EqualsEquals(Box<ExpressionAST>, Box<ExpressionAST>),
    NotEquals(Box<ExpressionAST>, Box<ExpressionAST>),
    Less(Box<ExpressionAST>, Box<ExpressionAST>),
    LessEquals(Box<ExpressionAST>, Box<ExpressionAST>),
    Greater(Box<ExpressionAST>, Box<ExpressionAST>),
    GreaterEquals(Box<ExpressionAST>, Box<ExpressionAST>),
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::grammar::ProgramParser;

    #[test]
    fn parse1() {
        let mut interner = Interner::new();
        let program = "fn basic(x, y) { return x + y; }";
        assert_eq!(
            format!(
                "{:?}",
                ProgramParser::new().parse(&mut interner, &program).unwrap()
            ),
            "ProgramAST { funcs: [FunctionAST { name: SymbolU16 { value: 3 }, params: [SymbolU16 { value: 1 }, SymbolU16 { value: 2 }], block: BlockAST { stmts: [Return(Add(Variable(SymbolU16 { value: 1 }), Variable(SymbolU16 { value: 2 })))] } }] }"
        );
    }

    #[test]
    fn parse2() {
        let mut interner = Interner::new();
        let program = "fn branch(x, y) { if x > y { x = y - 5; } else { y = x + 3; if y < x { y = y + 1; } } return x % y; }";
        assert_eq!(
            format!(
                "{:?}",
                ProgramParser::new().parse(&mut interner, &program).unwrap()
            ),
            "ProgramAST { funcs: [FunctionAST { name: SymbolU16 { value: 3 }, params: [SymbolU16 { value: 1 }, SymbolU16 { value: 2 }], block: BlockAST { stmts: [IfElse(Greater(Variable(SymbolU16 { value: 1 }), Variable(SymbolU16 { value: 2 })), BlockAST { stmts: [Assign(SymbolU16 { value: 1 }, Subtract(Variable(SymbolU16 { value: 2 }), NumberLiteral(5)))] }, Some(BlockAST { stmts: [Assign(SymbolU16 { value: 2 }, Add(Variable(SymbolU16 { value: 1 }), NumberLiteral(3))), IfElse(Less(Variable(SymbolU16 { value: 2 }), Variable(SymbolU16 { value: 1 })), BlockAST { stmts: [Assign(SymbolU16 { value: 2 }, Add(Variable(SymbolU16 { value: 2 }), NumberLiteral(1)))] }, None)] })), Return(Modulo(Variable(SymbolU16 { value: 1 }), Variable(SymbolU16 { value: 2 })))] } }] }"
        );
    }

    #[test]
    fn parse3() {
        let mut interner = Interner::new();
        let program = "fn loop(x) { while x > 0 { x = x - 1; } return x; }";
        assert_eq!(
            format!(
                "{:?}",
                ProgramParser::new().parse(&mut interner, &program).unwrap()
            ),
            "ProgramAST { funcs: [FunctionAST { name: SymbolU16 { value: 2 }, params: [SymbolU16 { value: 1 }], block: BlockAST { stmts: [While(Greater(Variable(SymbolU16 { value: 1 }), NumberLiteral(0)), BlockAST { stmts: [Assign(SymbolU16 { value: 1 }, Subtract(Variable(SymbolU16 { value: 1 }), NumberLiteral(1)))] }), Return(Variable(SymbolU16 { value: 1 }))] } }] }"
        );
    }
}
