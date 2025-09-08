use string_interner::symbol::SymbolU16;

pub type Symbol = SymbolU16;

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
