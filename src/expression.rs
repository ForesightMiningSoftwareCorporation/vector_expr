/// Top-level parseable calculation.
#[derive(Clone, Debug)]
pub enum Expression {
    Boolean(BoolExpression),
    Real(RealExpression),
}

/// A `bool`-valued expression.
#[derive(Clone, Debug)]
pub enum BoolExpression {
    // Binary logic.
    And(Box<BoolExpression>, Box<BoolExpression>),
    Or(Box<BoolExpression>, Box<BoolExpression>),

    // Unary logic.
    Not(Box<BoolExpression>),

    // Real comparisons.
    Equal(Box<RealExpression>, Box<RealExpression>),
    Greater(Box<RealExpression>, Box<RealExpression>),
    GreaterEqual(Box<RealExpression>, Box<RealExpression>),
    Less(Box<RealExpression>, Box<RealExpression>),
    LessEqual(Box<RealExpression>, Box<RealExpression>),
    NotEqual(Box<RealExpression>, Box<RealExpression>),
}

/// An `f64`-valued expression.
#[derive(Clone, Debug)]
pub enum RealExpression {
    // Binary real ops.
    Add(Box<RealExpression>, Box<RealExpression>),
    Div(Box<RealExpression>, Box<RealExpression>),
    Mul(Box<RealExpression>, Box<RealExpression>),
    Pow(Box<RealExpression>, Box<RealExpression>),
    Sub(Box<RealExpression>, Box<RealExpression>),

    // Unary real ops.
    Neg(Box<RealExpression>),

    // Constant.
    Literal(f64),

    // Input variable.
    Binding(BindingId),
}

/// Index into the `&[&[f64]]` bindings passed to expression evaluation.
pub type BindingId = usize;
