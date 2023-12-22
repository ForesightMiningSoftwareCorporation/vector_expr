# vector-expr

Vectorized math expression parser/evaluator.

## Why?

Performance. Evaluation of math expressions involving many variables can
incur significant overhead from traversing the expression tree or performing
variable lookups. We amortize that cost by performing intermediate
operations on _vectors_ of input data at a time (with optional data
parallelism via the `rayon` feature).

## Example

See unit tests in `src/lib.rs`.

License: MIT OR Apache-2.0
