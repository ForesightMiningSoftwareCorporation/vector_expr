use crate::expression::{BindingId, BoolExpression, Expression, RealExpression};
use crate::StringExpression;
use num_traits::Float;
use once_cell::sync::Lazy;
use pest::iterators::Pairs;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashSet;
use std::str::FromStr;

#[derive(Parser)]
#[grammar = "grammar.pest"] // relative to project `src`
struct ExpressionParser;

// Boxed because error is much larger than Ok variant in most results.
pub type ParseError = Box<pest::error::Error<Rule>>;

impl<Real: Float + FromStr> Expression<Real> {
    /// Assume this expression is real-valued.
    pub fn unwrap_real(self) -> RealExpression<Real> {
        match self {
            Self::Real(r) => r,
            _ => panic!("Expected Real"),
        }
    }

    /// Assume this expression is string-valued.
    pub fn unwrap_string(self) -> StringExpression {
        match self {
            Self::String(s) => s,
            _ => panic!("Expected String"),
        }
    }

    /// Assume this expression is boolean-valued.
    pub fn unwrap_bool(self) -> BoolExpression<Real> {
        match self {
            Self::Boolean(b) => b,
            _ => panic!("Expected Boolean"),
        }
    }

    pub fn parse_real_variable_names(input: &str) -> Result<HashSet<String>, ParseError> {
        Ok(ExpressionParser::parse(Rule::calculation, input)?
            .flatten()
            .filter(|p| (p.as_rule() == Rule::real_variable))
            .map(|p| p.as_str().to_string())
            .collect())
    }

    pub fn parse_string_variable_names(input: &str) -> Result<HashSet<String>, ParseError> {
        Ok(ExpressionParser::parse(Rule::calculation, input)?
            .flatten()
            .filter(|p| (p.as_rule() == Rule::str_variable))
            .map(|p| p.as_str().to_string())
            .collect())
    }

    /// Parse the expression from `input`.
    ///
    /// `binding_map` determines which variable name maps to each data binding.
    /// As variable names are encountered during parsing, they are replaced by
    /// [`BindingId`]s in the [`Expression`] syntax tree. This allows the
    /// [`Expression`] to be efficiently reused with many different data
    /// bindings.
    pub fn parse(input: &str, binding_map: impl Fn(&str) -> BindingId) -> Result<Self, ParseError> {
        let mut pairs = ExpressionParser::parse(Rule::calculation, input)?;
        // HACK: Working around https://github.com/pest-parser/pest/issues/943
        let inner_expr = pairs.next().unwrap().into_inner();
        Ok(parse_recursive(inner_expr, &binding_map))
    }
}

static PRATT_PARSER: Lazy<PrattParser<Rule>> = Lazy::new(|| {
    use Assoc::*;
    use Rule::*;

    PrattParser::new()
        .op(Op::infix(and, Left) | Op::infix(or, Left))
        .op(Op::infix(str_eq, Left)
            | Op::infix(str_neq, Left)
            | Op::infix(real_eq, Left)
            | Op::infix(real_neq, Left)
            | Op::infix(less, Left)
            | Op::infix(le, Left)
            | Op::infix(greater, Left)
            | Op::infix(ge, Left))
        .op(Op::infix(add, Left) | Op::infix(subtract, Left))
        .op(Op::infix(multiply, Left) | Op::infix(divide, Left))
        .op(Op::infix(power, Right))
});

fn parse_recursive<Real: FromStr + Float>(
    pairs: Pairs<Rule>,
    binding_map: &impl Fn(&str) -> BindingId,
) -> Expression<Real> {
    PRATT_PARSER
        .map_primary(|pair| match pair.as_rule() {
            Rule::bool_expr => parse_recursive(pair.into_inner(), binding_map),
            Rule::real_expr => parse_recursive(pair.into_inner(), binding_map),
            Rule::string_expr => parse_recursive(pair.into_inner(), binding_map),
            Rule::real_literal => {
                let literal_str = pair.as_str();
                if let Ok(value) = literal_str.parse::<Real>() {
                    return Expression::Real(RealExpression::Literal(value));
                }
                panic!("Unexpected literal: {}", literal_str)
            }
            Rule::string_literal => parse_recursive(pair.into_inner(), binding_map),
            Rule::string_literal_value => {
                let literal_str = pair.as_str();
                if let Ok(value) = literal_str.parse::<String>() {
                    return Expression::String(StringExpression::Literal(value));
                }
                panic!("Unexpected literal: {}", literal_str)
            }
            Rule::unary_real_op_expr => {
                let mut inner = pair.into_inner();
                let unary = inner.next().unwrap();
                match unary.as_rule() {
                    Rule::neg => Expression::Real(RealExpression::Neg(Box::new(
                        parse_recursive(inner, binding_map).unwrap_real(),
                    ))),
                    x => panic!("Unexpected unary logic operator: {x:?}"),
                }
            }
            Rule::unary_logic_expr => {
                let mut inner = pair.into_inner();
                let unary = inner.next().unwrap();
                match unary.as_rule() {
                    Rule::not => Expression::Boolean(BoolExpression::Not(Box::new(
                        parse_recursive(inner, binding_map).unwrap_bool(),
                    ))),
                    x => panic!("Unexpected unary logic operator: {x:?}"),
                }
            }
            Rule::real_variable => {
                Expression::Real(RealExpression::Binding(binding_map(pair.as_str())))
            }
            Rule::str_variable => {
                Expression::String(StringExpression::Binding(binding_map(pair.as_str())))
            }
            x => panic!("Unexpected primary rule {x:?}"),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::add => Expression::Real(RealExpression::Add(
                Box::new(lhs.unwrap_real()),
                Box::new(rhs.unwrap_real()),
            )),
            Rule::subtract => Expression::Real(RealExpression::Sub(
                Box::new(lhs.unwrap_real()),
                Box::new(rhs.unwrap_real()),
            )),
            Rule::multiply => Expression::Real(RealExpression::Mul(
                Box::new(lhs.unwrap_real()),
                Box::new(rhs.unwrap_real()),
            )),
            Rule::divide => Expression::Real(RealExpression::Div(
                Box::new(lhs.unwrap_real()),
                Box::new(rhs.unwrap_real()),
            )),
            Rule::power => Expression::Real(RealExpression::Pow(
                Box::new(lhs.unwrap_real()),
                Box::new(rhs.unwrap_real()),
            )),
            Rule::real_eq => Expression::Boolean(BoolExpression::Equal(
                Box::new(lhs.unwrap_real()),
                Box::new(rhs.unwrap_real()),
            )),
            Rule::real_neq => Expression::Boolean(BoolExpression::NotEqual(
                Box::new(lhs.unwrap_real()),
                Box::new(rhs.unwrap_real()),
            )),
            Rule::str_eq => Expression::Boolean(BoolExpression::StrEqual(
                lhs.unwrap_string(),
                rhs.unwrap_string(),
            )),
            Rule::str_neq => Expression::Boolean(BoolExpression::StrNotEqual(
                lhs.unwrap_string(),
                rhs.unwrap_string(),
            )),
            Rule::less => Expression::Boolean(BoolExpression::Less(
                Box::new(lhs.unwrap_real()),
                Box::new(rhs.unwrap_real()),
            )),
            Rule::le => Expression::Boolean(BoolExpression::LessEqual(
                Box::new(lhs.unwrap_real()),
                Box::new(rhs.unwrap_real()),
            )),
            Rule::greater => Expression::Boolean(BoolExpression::Greater(
                Box::new(lhs.unwrap_real()),
                Box::new(rhs.unwrap_real()),
            )),
            Rule::ge => Expression::Boolean(BoolExpression::GreaterEqual(
                Box::new(lhs.unwrap_real()),
                Box::new(rhs.unwrap_real()),
            )),
            Rule::and => Expression::Boolean(BoolExpression::And(
                Box::new(lhs.unwrap_bool()),
                Box::new(rhs.unwrap_bool()),
            )),
            Rule::or => Expression::Boolean(BoolExpression::Or(
                Box::new(lhs.unwrap_bool()),
                Box::new(rhs.unwrap_bool()),
            )),
            x => panic!("Unexpected operator {x:?}"),
        })
        .parse(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_variable_names() {
        let vars = Expression::<f32>::parse_real_variable_names("v1_dest + x + y + z99").unwrap();
        assert!(vars.contains("x"), "{vars:?}");
        assert!(vars.contains("y"), "{vars:?}");
        assert!(vars.contains("z99"), "{vars:?}");
        assert!(vars.contains("v1_dest"), "{vars:?}");
        let vars = Expression::<f32>::parse_string_variable_names("x == \"W\"").unwrap();
        assert!(vars.contains("x"), "{vars:?}");
    }

    #[test]
    fn parse_comparisons() {
        fn binding_map(var_name: &str) -> BindingId {
            match var_name {
                "x" => 0,
                "y" => 1,
                _ => unreachable!(),
            }
        }
        Expression::<f32>::parse("x == y", binding_map).unwrap();
        Expression::<f32>::parse("x != y", binding_map).unwrap();
        Expression::<f32>::parse("x > y", binding_map).unwrap();
        Expression::<f32>::parse("x < y", binding_map).unwrap();
        Expression::<f32>::parse("x <= y", binding_map).unwrap();
        Expression::<f32>::parse("x >= y", binding_map).unwrap();
    }
}
