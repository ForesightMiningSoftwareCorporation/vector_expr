# vector-expr

Vectorized math expression parser/evaluator.

## Why?

Performance. Evaluation of math expressions involving many variables can
incur significant overhead from traversing the expression tree or performing
variable lookups. We amortize that cost by performing intermediate
operations on _vectors_ of input data at a time (with optional data
parallelism via the `rayon` feature).

## Example

```rust
use vector_expr::*;

fn binding_map(var_name: &str) -> BindingId {
    match var_name {
        "bar" => 0,
        "baz" => 1,
        "foo" => 2,
        _ => unreachable!(),
    }
}
let parsed = Expression::parse("2 * (foo + bar) * baz", &binding_map).unwrap();
let real = parsed.unwrap_real();

let bar = [1.0, 2.0, 3.0];
let baz = [4.0, 5.0, 6.0];
let foo = [7.0, 8.0, 9.0];
let bindings: &[&[f64]] = &[&bar, &baz, &foo];
let mut registers = Registers::new(3);
let output = real.evaluate(bindings, &mut registers);
assert_eq!(&output, &[64.0, 100.0, 144.0]);
```

License: MIT OR Apache-2.0
