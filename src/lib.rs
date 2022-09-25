//! Vectorized math expression parser/evaluator.
//!
//! # Why?
//!
//! Performance. Evaluation of math expressions involving many variables can
//! incur significant overhead from traversing the expression tree or performing
//! variable lookups. We amortize that cost by performing intermediate
//! operations on _vectors_ of input data at a time (with optional data
//! parallelism via the `rayon` feature).
//!
//! # Example
//!
//! ```rust
//! use vector_expr::*;
//!
//! fn binding_map(var_name: &str) -> BindingId {
//!     match var_name {
//!         "bar" => 0,
//!         "baz" => 1,
//!         "foo" => 2,
//!         _ => unreachable!(),
//!     }
//! }
//! let parsed = Expression::parse("2 * (foo + bar) * baz", &binding_map).unwrap();
//! let real = parsed.unwrap_real();
//!
//! let bar = [1.0, 2.0, 3.0];
//! let baz = [4.0, 5.0, 6.0];
//! let foo = [7.0, 8.0, 9.0];
//! let bindings: &[&[f64]] = &[&bar, &baz, &foo];
//! let mut registers = Registers::new(3);
//! let output = real.evaluate(bindings, &mut registers);
//! assert_eq!(&output, &[64.0, 100.0, 144.0]);
//! ```

mod evaluate;
mod expression;
mod parse;

/// Uses the [`pest`] parsing expression grammar language.
///
/// ```text
#[doc = include_str!("grammar.pest")]
/// ```
pub mod grammar_doc {}

pub use evaluate::*;
pub use expression::*;
pub use parse::ParseError;

/// Pass to `Expression::parse` if the expression has no variables.
pub fn empty_binding_map(_var_name: &str) -> BindingId {
    panic!("Empty binding map")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_expression() {
        fn binding_map(var_name: &str) -> BindingId {
            match var_name {
                "bar" => 0,
                "baz" => 1,
                "foo" => 2,
                _ => unreachable!(),
            }
        }
        let parsed = Expression::parse("2 * (foo + bar) * -baz", &binding_map).unwrap();
        let real = parsed.unwrap_real();

        let bar = [1.0, 2.0, 3.0];
        let baz = [4.0, 5.0, 6.0];
        let foo = [7.0, 8.0, 9.0];
        let bindings: &[&[f64]] = &[&bar, &baz, &foo];
        let mut registers = Registers::new(3);
        let output = real.evaluate(bindings, &mut registers);
        assert_eq!(&output, &[-64.0, -100.0, -144.0]);
        assert_eq!(registers.num_allocations(), 3);
    }

    #[test]
    fn real_op_precedence() {
        let mut registers = Registers::new(1);

        let parsed = Expression::parse("1 * 2 + 3 * 4", &empty_binding_map).unwrap();
        let real = parsed.unwrap_real();
        let output = real.evaluate(&[], &mut registers);
        assert_eq!(&output, &[14.0]);

        let parsed = Expression::parse("8 / 4 * 3", &empty_binding_map).unwrap();
        let real = parsed.unwrap_real();
        let output = real.evaluate(&[], &mut registers);
        assert_eq!(&output, &[6.0]);

        let parsed = Expression::parse("4 ^ 3 ^ 2", &empty_binding_map).unwrap();
        let real = parsed.unwrap_real();
        let output = real.evaluate(&[], &mut registers);
        assert_eq!(&output, &[262144.0]);
    }

    #[test]
    fn bool_expression() {
        fn binding_map(var_name: &str) -> BindingId {
            match var_name {
                "bar" => 0,
                "baz" => 1,
                "foo" => 2,
                _ => unreachable!(),
            }
        }
        let parsed = Expression::parse("!(bar < foo && bar < baz)", &binding_map).unwrap();
        let bool = parsed.unwrap_bool();

        let bar = [1.0, 6.0, 7.0];
        let baz = [2.0, 5.0, 8.0];
        let foo = [3.0, 4.0, 9.0];
        let bindings: &[&[f64]] = &[&bar, &baz, &foo];
        let mut registers = Registers::new(3);
        let output = bool.evaluate(bindings, &mut registers);
        assert_eq!(&output, &[false, true, false]);
        assert_eq!(registers.num_allocations(), 3);
    }

    #[test]
    fn naive_allocations_limited_by_recycling() {
        fn binding_map(var_name: &str) -> BindingId {
            match var_name {
                "bar" => 0,
                "baz" => 1,
                "foo" => 2,
                _ => unreachable!(),
            }
        }
        let parsed = Expression::parse(
            "foo + bar + baz + foo + bar + baz + foo + bar + baz",
            &binding_map,
        )
        .unwrap();
        let real = parsed.unwrap_real();

        let bar = [1.0, 2.0, 3.0];
        let baz = [4.0, 5.0, 6.0];
        let foo = [7.0, 8.0, 9.0];
        let bindings: &[&[f64]] = &[&bar, &baz, &foo];
        let mut registers = Registers::new(3);
        let output = real.evaluate(bindings, &mut registers);
        assert_eq!(&output, &[36.0, 45.0, 54.0]);
        assert_eq!(registers.num_allocations(), 2);
    }

    #[test]
    fn real_bench() {
        fn binding_map(var_name: &str) -> BindingId {
            match var_name {
                "x" => 0,
                "y" => 1,
                "z" => 2,
                var => panic!("Unexpected variable: {var}"),
            }
        }
        let parsed = Expression::parse("(z + (z^2 - 4*x*y)^0.5) / (2*x)", &binding_map).unwrap();
        let real = parsed.unwrap_real();

        const LEN: i32 = 10_000_000;
        let x: Vec<_> = (0..LEN).map(|i| i as f64).collect();
        let y: Vec<_> = (0..LEN).map(|i| (LEN - i) as f64).collect();
        let z: Vec<_> = (0..LEN).map(|i| ((LEN / 2) - i) as f64).collect();
        let bindings: &[&[f64]] = &[&x, &y, &z];

        let mut registers = Registers::new(LEN as usize);
        let start = std::time::Instant::now();
        let _output = real.evaluate(bindings, &mut registers);
        let elapsed = start.elapsed().as_millis();
        println!(
            "Took {elapsed} ms, {} ns per element",
            (1_000_000 * elapsed) / LEN as u128
        );
        assert_eq!(registers.num_allocations(), 3);
    }
}
