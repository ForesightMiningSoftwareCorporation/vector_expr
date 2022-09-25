use crate::{BoolExpression, RealExpression};

#[cfg(feature = "rayon")]
use rayon::prelude::{
    IndexedParallelIterator, IntoParallelRefIterator, ParallelExtend, ParallelIterator,
};

impl BoolExpression {
    /// Calculates the `bool`-valued results of the expression component-wise.
    pub fn evaluate(&self, bindings: &[&[f64]], registers: &mut Registers) -> Vec<bool> {
        validate_bindings(bindings, registers.register_length);
        self.evaluate_recursive(bindings, registers)
    }

    fn evaluate_recursive(&self, bindings: &[&[f64]], registers: &mut Registers) -> Vec<bool> {
        match self {
            Self::And(lhs, rhs) => evaluate_binary_logic(
                |lhs, rhs| lhs && rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
            Self::Equal(lhs, rhs) => evaluate_comparison(
                |lhs, rhs| lhs == rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
            Self::Greater(lhs, rhs) => evaluate_comparison(
                |lhs, rhs| lhs > rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
            Self::GreaterEqual(lhs, rhs) => evaluate_comparison(
                |lhs, rhs| lhs >= rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
            Self::Less(lhs, rhs) => evaluate_comparison(
                |lhs, rhs| lhs < rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
            Self::LessEqual(lhs, rhs) => evaluate_comparison(
                |lhs, rhs| lhs <= rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
            Self::Not(only) => {
                evaluate_unary_logic(|only| !only, only.as_ref(), bindings, registers)
            }
            Self::NotEqual(lhs, rhs) => evaluate_comparison(
                |lhs, rhs| lhs != rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
            Self::Or(lhs, rhs) => evaluate_binary_logic(
                |lhs, rhs| lhs || rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
        }
    }
}

impl RealExpression {
    /// Calculates the real-valued results of the expression component-wise.
    pub fn evaluate(&self, bindings: &[&[f64]], registers: &mut Registers) -> Vec<f64> {
        validate_bindings(bindings, registers.register_length);
        self.evaluate_recursive(bindings, registers)
    }

    fn evaluate_recursive(&self, bindings: &[&[f64]], registers: &mut Registers) -> Vec<f64> {
        match self {
            Self::Add(lhs, rhs) => evaluate_binary_real_op(
                |lhs, rhs| lhs + rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
            // This branch should only be taken if the entire expression is
            // literally the identity map from one of the bindings.
            Self::Binding(binding) => {
                let mut output = registers.allocate_real();
                output.extend_from_slice(bindings[*binding]);
                output
            }
            Self::Div(lhs, rhs) => evaluate_binary_real_op(
                |lhs, rhs| lhs / rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
            Self::Literal(value) => {
                let mut output = registers.allocate_real();
                output.extend(std::iter::repeat(*value).take(registers.register_length));
                output
            }
            Self::Mul(lhs, rhs) => evaluate_binary_real_op(
                |lhs, rhs| lhs * rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
            Self::Neg(only) => {
                evaluate_unary_real_op(|only| -only, only.as_ref(), bindings, registers)
            }
            Self::Pow(lhs, rhs) => evaluate_binary_real_op(
                |lhs, rhs| lhs.powf(rhs),
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
            Self::Sub(lhs, rhs) => evaluate_binary_real_op(
                |lhs, rhs| lhs - rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                bindings,
                registers,
            ),
        }
    }
}

fn validate_bindings(input_bindings: &[&[f64]], expected_length: usize) {
    for b in input_bindings.iter() {
        assert_eq!(b.len(), expected_length);
    }
}

#[inline]
fn evaluate_binary_real_op(
    op: fn(f64, f64) -> f64,
    lhs: &RealExpression,
    rhs: &RealExpression,
    bindings: &[&[f64]],
    registers: &mut Registers,
) -> Vec<f64> {
    // Before doing recursive evaluation, we check first if we already have
    // input values in our bindings. This avoids unnecessary copies.
    let mut lhs_reg = None;
    let lhs_values = if let RealExpression::Binding(binding) = lhs {
        bindings[*binding]
    } else {
        lhs_reg = Some(lhs.evaluate_recursive(bindings, registers));
        lhs_reg.as_ref().unwrap()
    };
    let mut rhs_reg = None;
    let rhs_values = if let RealExpression::Binding(binding) = rhs {
        bindings[*binding]
    } else {
        rhs_reg = Some(rhs.evaluate_recursive(bindings, registers));
        rhs_reg.as_ref().unwrap()
    };
    // Allocate this output register as lazily as possible.
    let mut output = registers.allocate_real();

    #[cfg(feature = "rayon")]
    {
        output.par_extend(
            lhs_values
                .par_iter()
                .zip(rhs_values.par_iter())
                .map(|(lhs, rhs)| op(*lhs, *rhs)),
        );
    }
    #[cfg(not(feature = "rayon"))]
    {
        output.extend(
            lhs_values
                .iter()
                .zip(rhs_values.iter())
                .map(|(lhs, rhs)| op(*lhs, *rhs)),
        );
    }

    if let Some(r) = lhs_reg {
        registers.recycle_real(r);
    }
    if let Some(r) = rhs_reg {
        registers.recycle_real(r);
    }
    output
}

#[inline]
fn evaluate_unary_real_op(
    op: fn(f64) -> f64,
    only: &RealExpression,
    bindings: &[&[f64]],
    registers: &mut Registers,
) -> Vec<f64> {
    // Before doing recursive evaluation, we check first if we already have
    // input values in our bindings. This avoids unnecessary copies.
    let mut only_reg = None;
    let only_values = if let RealExpression::Binding(binding) = only {
        bindings[*binding]
    } else {
        only_reg = Some(only.evaluate_recursive(bindings, registers));
        only_reg.as_ref().unwrap()
    };
    // Allocate this output register as lazily as possible.
    let mut output = registers.allocate_real();

    #[cfg(feature = "rayon")]
    {
        output.par_extend(only_values.par_iter().map(|only| op(*only)));
    }
    #[cfg(not(feature = "rayon"))]
    {
        output.extend(only_values.iter().map(|only| op(*only)));
    }

    if let Some(r) = only_reg {
        registers.recycle_real(r);
    }
    output
}

#[inline]
fn evaluate_comparison(
    op: fn(f64, f64) -> bool,
    lhs: &RealExpression,
    rhs: &RealExpression,
    bindings: &[&[f64]],
    registers: &mut Registers,
) -> Vec<bool> {
    // Before doing recursive evaluation, we check first if we already have
    // input values in our bindings. This avoids unnecessary copies.
    let mut lhs_reg = None;
    let lhs_values = if let RealExpression::Binding(binding) = lhs {
        bindings[*binding]
    } else {
        lhs_reg = Some(lhs.evaluate_recursive(bindings, registers));
        lhs_reg.as_ref().unwrap()
    };
    let mut rhs_reg = None;
    let rhs_values = if let RealExpression::Binding(binding) = rhs {
        bindings[*binding]
    } else {
        rhs_reg = Some(rhs.evaluate_recursive(bindings, registers));
        rhs_reg.as_ref().unwrap()
    };
    // Allocate this output register as lazily as possible.
    let mut output = registers.allocate_bool();

    #[cfg(feature = "rayon")]
    {
        output.par_extend(
            lhs_values
                .par_iter()
                .zip(rhs_values.par_iter())
                .map(|(lhs, rhs)| op(*lhs, *rhs)),
        );
    }
    #[cfg(not(feature = "rayon"))]
    {
        output.extend(
            lhs_values
                .iter()
                .zip(rhs_values.iter())
                .map(|(lhs, rhs)| op(*lhs, *rhs)),
        );
    }

    if let Some(r) = lhs_reg {
        registers.recycle_real(r);
    }
    if let Some(r) = rhs_reg {
        registers.recycle_real(r);
    }
    output
}

#[inline]
fn evaluate_binary_logic(
    op: fn(bool, bool) -> bool,
    lhs: &BoolExpression,
    rhs: &BoolExpression,
    bindings: &[&[f64]],
    registers: &mut Registers,
) -> Vec<bool> {
    let lhs_values = lhs.evaluate_recursive(bindings, registers);
    let rhs_values = rhs.evaluate_recursive(bindings, registers);

    // Allocate this output register as lazily as possible.
    let mut output = registers.allocate_bool();

    #[cfg(feature = "rayon")]
    {
        output.par_extend(
            lhs_values
                .par_iter()
                .zip(rhs_values.par_iter())
                .map(|(lhs, rhs)| op(*lhs, *rhs)),
        );
    }
    #[cfg(not(feature = "rayon"))]
    {
        output.extend(
            lhs_values
                .iter()
                .zip(rhs_values.iter())
                .map(|(lhs, rhs)| op(*lhs, *rhs)),
        );
    }

    registers.recycle_bool(lhs_values);
    registers.recycle_bool(rhs_values);
    output
}

#[inline]
fn evaluate_unary_logic(
    op: fn(bool) -> bool,
    only: &BoolExpression,
    bindings: &[&[f64]],
    registers: &mut Registers,
) -> Vec<bool> {
    let only_values = only.evaluate_recursive(bindings, registers);

    // Allocate this output register as lazily as possible.
    let mut output = registers.allocate_bool();

    #[cfg(feature = "rayon")]
    {
        output.par_extend(only_values.par_iter().map(|only| op(*only)));
    }
    #[cfg(not(feature = "rayon"))]
    {
        output.extend(only_values.iter().map(|only| op(*only)));
    }

    registers.recycle_bool(only_values);
    output
}

/// Scratch space for calculations. Can be reused across evaluations with the
/// same data binding length.
///
/// Attempts to minimize allocations by recycling registers after intermediate
/// calculations have finished.
pub struct Registers {
    num_allocations: usize,
    real_registers: Vec<Vec<f64>>,
    bool_registers: Vec<Vec<bool>>,
    register_length: usize,
}

impl Registers {
    pub fn new(register_length: usize) -> Self {
        Self {
            num_allocations: 0,
            real_registers: vec![],
            bool_registers: vec![],
            register_length,
        }
    }

    fn recycle_real(&mut self, mut used: Vec<f64>) {
        used.clear();
        self.real_registers.push(used);
    }

    fn recycle_bool(&mut self, mut used: Vec<bool>) {
        used.clear();
        self.bool_registers.push(used);
    }

    fn allocate_real(&mut self) -> Vec<f64> {
        self.real_registers.pop().unwrap_or_else(|| {
            self.num_allocations += 1;
            Vec::with_capacity(self.register_length)
        })
    }

    fn allocate_bool(&mut self) -> Vec<bool> {
        self.bool_registers.pop().unwrap_or_else(|| {
            self.num_allocations += 1;
            Vec::with_capacity(self.register_length)
        })
    }

    pub fn num_allocations(&self) -> usize {
        self.num_allocations
    }
}
