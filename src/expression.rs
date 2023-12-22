/// Top-level parseable calculation.
#[derive(Clone, Debug)]
pub enum Expression<Real> {
    Boolean(BoolExpression<Real>),
    Real(RealExpression<Real>),
    String(StringExpression),
}

/// A `bool`-valued expression.
#[derive(Clone, Debug)]
pub enum BoolExpression<Real> {
    // Binary logic.
    And(Box<BoolExpression<Real>>, Box<BoolExpression<Real>>),
    Or(Box<BoolExpression<Real>>, Box<BoolExpression<Real>>),

    // Unary logic.
    Not(Box<BoolExpression<Real>>),

    // Real comparisons.
    Equal(Box<RealExpression<Real>>, Box<RealExpression<Real>>),
    Greater(Box<RealExpression<Real>>, Box<RealExpression<Real>>),
    GreaterEqual(Box<RealExpression<Real>>, Box<RealExpression<Real>>),
    Less(Box<RealExpression<Real>>, Box<RealExpression<Real>>),
    LessEqual(Box<RealExpression<Real>>, Box<RealExpression<Real>>),
    NotEqual(Box<RealExpression<Real>>, Box<RealExpression<Real>>),

    // String comparisons.
    StrEqual(StringExpression, StringExpression),
    StrNotEqual(StringExpression, StringExpression),
}

/// An `f64`-valued expression.
#[derive(Clone, Debug)]
pub enum RealExpression<Real> {
    // Binary real ops.
    Add(Box<RealExpression<Real>>, Box<RealExpression<Real>>),
    Div(Box<RealExpression<Real>>, Box<RealExpression<Real>>),
    Mul(Box<RealExpression<Real>>, Box<RealExpression<Real>>),
    Pow(Box<RealExpression<Real>>, Box<RealExpression<Real>>),
    Sub(Box<RealExpression<Real>>, Box<RealExpression<Real>>),

    // Unary real ops.
    Neg(Box<RealExpression<Real>>),

    // Constant.
    Literal(Real),

    // Input variable.
    Binding(BindingId),
}

#[derive(Clone, Debug)]
pub enum StringExpression {
    Literal(String),
    Binding(BindingId),
}

/// Index into the `&[&[f64]]` bindings passed to expression evaluation.
pub type BindingId = usize;
