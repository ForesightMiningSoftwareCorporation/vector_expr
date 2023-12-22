use crate::real::Real;
use crate::{BoolExpression, RealExpression, StringExpression};

#[cfg(feature = "rayon")]
use rayon::prelude::{
    IndexedParallelIterator, IntoParallelRefIterator, ParallelExtend, ParallelIterator,
};

/// To speed up string comparisons, we use string interning.
pub type StringId = u32;

impl BoolExpression {
    /// Calculates the `bool`-valued results of the expression component-wise.
    pub fn evaluate<R: AsRef<[Real]>, S: AsRef<[StringId]>>(
        &self,
        real_bindings: &[R],
        string_bindings: &[S],
        mut get_string_literal_id: impl FnMut(&str) -> StringId,
        registers: &mut Registers,
    ) -> Vec<bool> {
        validate_bindings(real_bindings, registers.register_length);
        validate_bindings(string_bindings, registers.register_length);
        self.evaluate_recursive(
            real_bindings,
            string_bindings,
            &mut get_string_literal_id,
            registers,
        )
    }

    fn evaluate_recursive<R: AsRef<[Real]>, S: AsRef<[StringId]>>(
        &self,
        real_bindings: &[R],
        string_bindings: &[S],
        get_string_literal_id: &mut impl FnMut(&str) -> StringId,
        registers: &mut Registers,
    ) -> Vec<bool> {
        match self {
            Self::And(lhs, rhs) => evaluate_binary_logic(
                |lhs, rhs| lhs && rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                real_bindings,
                string_bindings,
                get_string_literal_id,
                registers,
            ),
            Self::Equal(lhs, rhs) => evaluate_real_comparison(
                |lhs, rhs| lhs == rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                real_bindings,
                registers,
            ),
            Self::Greater(lhs, rhs) => evaluate_real_comparison(
                |lhs, rhs| lhs > rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                real_bindings,
                registers,
            ),
            Self::GreaterEqual(lhs, rhs) => evaluate_real_comparison(
                |lhs, rhs| lhs >= rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                real_bindings,
                registers,
            ),
            Self::Less(lhs, rhs) => evaluate_real_comparison(
                |lhs, rhs| lhs < rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                real_bindings,
                registers,
            ),
            Self::LessEqual(lhs, rhs) => evaluate_real_comparison(
                |lhs, rhs| lhs <= rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                real_bindings,
                registers,
            ),
            Self::Not(only) => evaluate_unary_logic(
                |only| !only,
                only.as_ref(),
                real_bindings,
                string_bindings,
                get_string_literal_id,
                registers,
            ),
            Self::NotEqual(lhs, rhs) => evaluate_real_comparison(
                |lhs, rhs| lhs != rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                real_bindings,
                registers,
            ),
            Self::Or(lhs, rhs) => evaluate_binary_logic(
                |lhs, rhs| lhs || rhs,
                lhs.as_ref(),
                rhs.as_ref(),
                real_bindings,
                string_bindings,
                get_string_literal_id,
                registers,
            ),
            Self::StrEqual(lhs, rhs) => evaluate_string_comparison(
                |lhs, rhs| lhs == rhs,
                lhs,
                rhs,
                string_bindings,
                get_string_literal_id,
                registers,
            ),
            Self::StrNotEqual(lhs, rhs) => evaluate_string_comparison(
                |lhs, rhs| lhs != rhs,
                lhs,
                rhs,
                string_bindings,
                get_string_literal_id,
                registers,
            ),
        }
    }
}

impl RealExpression {
    pub fn evaluate_without_vars(&self, registers: &mut Registers) -> Vec<Real> {
        self.evaluate::<[_; 0]>(&[], registers)
    }

    /// Calculates the real-valued results of the expression component-wise.
    pub fn evaluate<R: AsRef<[Real]>>(
        &self,
        bindings: &[R],
        registers: &mut Registers,
    ) -> Vec<Real> {
        validate_bindings(bindings, registers.register_length);
        self.evaluate_recursive(bindings, registers)
    }

    fn evaluate_recursive<R: AsRef<[Real]>>(
        &self,
        bindings: &[R],
        registers: &mut Registers,
    ) -> Vec<Real> {
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
                output.extend_from_slice(bindings[*binding].as_ref());
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

fn validate_bindings<T, B: AsRef<[T]>>(input_bindings: &[B], expected_length: usize) {
    for b in input_bindings.iter() {
        assert_eq!(b.as_ref().len(), expected_length);
    }
}

fn evaluate_binary_real_op<R: AsRef<[Real]>>(
    op: fn(Real, Real) -> Real,
    lhs: &RealExpression,
    rhs: &RealExpression,
    bindings: &[R],
    registers: &mut Registers,
) -> Vec<Real> {
    // Before doing recursive evaluation, we check first if we already have
    // input values in our bindings. This avoids unnecessary copies.
    let mut lhs_reg = None;
    let lhs_values = if let RealExpression::Binding(binding) = lhs {
        bindings[*binding].as_ref()
    } else {
        lhs_reg = Some(lhs.evaluate_recursive(bindings, registers));
        lhs_reg.as_ref().unwrap()
    };
    let mut rhs_reg = None;
    let rhs_values = if let RealExpression::Binding(binding) = rhs {
        bindings[*binding].as_ref()
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

fn evaluate_unary_real_op<R: AsRef<[Real]>>(
    op: fn(Real) -> Real,
    only: &RealExpression,
    bindings: &[R],
    registers: &mut Registers,
) -> Vec<Real> {
    // Before doing recursive evaluation, we check first if we already have
    // input values in our bindings. This avoids unnecessary copies.
    let mut only_reg = None;
    let only_values = if let RealExpression::Binding(binding) = only {
        bindings[*binding].as_ref()
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

fn evaluate_real_comparison<R: AsRef<[Real]>>(
    op: fn(Real, Real) -> bool,
    lhs: &RealExpression,
    rhs: &RealExpression,
    bindings: &[R],
    registers: &mut Registers,
) -> Vec<bool> {
    // Before doing recursive evaluation, we check first if we already have
    // input values in our bindings. This avoids unnecessary copies.
    let mut lhs_reg = None;
    let lhs_values = if let RealExpression::Binding(binding) = lhs {
        bindings[*binding].as_ref()
    } else {
        lhs_reg = Some(lhs.evaluate_recursive(bindings, registers));
        lhs_reg.as_ref().unwrap()
    };
    let mut rhs_reg = None;
    let rhs_values = if let RealExpression::Binding(binding) = rhs {
        bindings[*binding].as_ref()
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

fn evaluate_string_comparison<S: AsRef<[StringId]>>(
    op: fn(StringId, StringId) -> bool,
    lhs: &StringExpression,
    rhs: &StringExpression,
    bindings: &[S],
    mut get_string_literal_id: impl FnMut(&str) -> StringId,
    registers: &mut Registers,
) -> Vec<bool> {
    let mut lhs_reg = None;
    let lhs_values = match lhs {
        StringExpression::Binding(binding) => bindings[*binding].as_ref(),
        StringExpression::Literal(literal_value) => {
            let mut reg = registers.allocate_string();
            let literal_id = get_string_literal_id(literal_value);
            reg.extend(std::iter::repeat(literal_id).take(registers.register_length));
            lhs_reg = Some(reg);
            lhs_reg.as_ref().unwrap()
        }
    };
    let mut rhs_reg = None;
    let rhs_values = match rhs {
        StringExpression::Binding(binding) => bindings[*binding].as_ref(),
        StringExpression::Literal(literal_value) => {
            let mut reg = registers.allocate_string();
            let literal_id = get_string_literal_id(literal_value);
            reg.extend(std::iter::repeat(literal_id).take(registers.register_length));
            rhs_reg = Some(reg);
            rhs_reg.as_ref().unwrap()
        }
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
        registers.recycle_string(r);
    }
    if let Some(r) = rhs_reg {
        registers.recycle_string(r);
    }
    output
}

fn evaluate_binary_logic<R: AsRef<[Real]>, S: AsRef<[StringId]>>(
    op: fn(bool, bool) -> bool,
    lhs: &BoolExpression,
    rhs: &BoolExpression,
    real_bindings: &[R],
    string_bindings: &[S],
    get_string_literal_id: &mut impl FnMut(&str) -> StringId,
    registers: &mut Registers,
) -> Vec<bool> {
    let lhs_values = lhs.evaluate_recursive(
        real_bindings,
        string_bindings,
        get_string_literal_id,
        registers,
    );
    let rhs_values = rhs.evaluate_recursive(
        real_bindings,
        string_bindings,
        get_string_literal_id,
        registers,
    );

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

fn evaluate_unary_logic<R: AsRef<[Real]>, S: AsRef<[StringId]>>(
    op: fn(bool) -> bool,
    only: &BoolExpression,
    real_bindings: &[R],
    string_bindings: &[S],
    get_string_literal_id: &mut impl FnMut(&str) -> StringId,
    registers: &mut Registers,
) -> Vec<bool> {
    let only_values = only.evaluate_recursive(
        real_bindings,
        string_bindings,
        get_string_literal_id,
        registers,
    );

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
    real_registers: Vec<Vec<Real>>,
    bool_registers: Vec<Vec<bool>>,
    string_registers: Vec<Vec<StringId>>,
    register_length: usize,
}

impl Registers {
    pub fn new(register_length: usize) -> Self {
        Self {
            num_allocations: 0,
            real_registers: vec![],
            bool_registers: vec![],
            string_registers: vec![],
            register_length,
        }
    }

    fn recycle_real(&mut self, mut used: Vec<Real>) {
        used.clear();
        self.real_registers.push(used);
    }

    fn recycle_bool(&mut self, mut used: Vec<bool>) {
        used.clear();
        self.bool_registers.push(used);
    }

    fn recycle_string(&mut self, mut used: Vec<StringId>) {
        used.clear();
        self.string_registers.push(used);
    }

    fn allocate_real(&mut self) -> Vec<Real> {
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

    fn allocate_string(&mut self) -> Vec<StringId> {
        self.string_registers.pop().unwrap_or_else(|| {
            self.num_allocations += 1;
            Vec::with_capacity(self.register_length)
        })
    }

    pub fn num_allocations(&self) -> usize {
        self.num_allocations
    }
}
