use crate::util::arithmetic::{CurveAffine, FieldExt};
use std::{fmt::Debug, ops::Deref};

/// Instructions to handle field element operations.
pub trait IntegerInstructions<F: FieldExt>: Clone + Debug {
    /// Context (either enhanced `region` or some kind of builder).
    type Context: Debug;
    /// Assigned cell.
    type AssignedCell: Clone + Debug;
    /// Assigned integer.
    type AssignedInteger: Clone + Debug;

    /// Assign an integer witness.
    fn assign_integer(
        &self,
        ctx: &mut Self::Context,
        integer: F, // witness
    ) -> Self::AssignedInteger;

    /// Assign an integer constant.
    fn assign_constant(&self, ctx: &mut Self::Context, integer: F) -> Self::AssignedInteger;

    /// Sum integers with coefficients and constant.
    fn sum_with_coeff_and_const(
        &self,
        ctx: &mut Self::Context,
        values: &[(F::Scalar, impl Deref<Target = Self::AssignedInteger>)],
        constant: F::Scalar,
    ) -> Self::AssignedInteger;

    /// Sum product of integers with coefficients and constant.
    fn sum_products_with_coeff_and_const(
        &self,
        ctx: &mut Self::Context,
        values: &[(
            F::Scalar,
            impl Deref<Target = Self::AssignedInteger>,
            impl Deref<Target = Self::AssignedInteger>,
        )],
        constant: F::Scalar,
    ) -> Self::AssignedInteger;

    /// Returns `lhs - rhs`.
    fn sub(
        &self,
        ctx: &mut Self::Context,
        lhs: &Self::AssignedInteger,
        rhs: &Self::AssignedInteger,
    ) -> Self::AssignedInteger;

    /// Returns `-value`.
    fn neg(&self, ctx: &mut Self::Context, value: &Self::AssignedInteger) -> Self::AssignedInteger;

    /// Returns `1/value`.
    fn invert(
        &self,
        ctx: &mut Self::Context,
        value: &Self::AssignedInteger,
    ) -> Self::AssignedInteger;

    /// Enforce `lhs` and `rhs` are equal.
    fn assert_equal(
        &self,
        ctx: &mut Self::Context,
        lhs: &Self::AssignedInteger,
        rhs: &Self::AssignedInteger,
    );
}

/// Instructions to handle elliptic curve point operations.
pub trait EccInstructions<C: CurveAffine>: Clone + Debug {
    /// Context
    type Context: Debug + Default;
    /// [`IntegerInstructions`] to handle scalar field operation.
    type ScalarChip: IntegerInstructions<
        C::Scalar,
        Context = Self::Context,
        AssignedCell = Self::AssignedCell,
        AssignedInteger = Self::AssignedScalar,
    >;
    /// Assigned cell.
    type AssignedCell: Clone + Debug;
    /// Assigned scalar field element.
    type AssignedScalar: Clone + Debug;
    /// Assigned elliptic curve point.
    type AssignedEcPoint: Clone + Debug;

    /// Returns reference of [`EccInstructions::ScalarChip`].
    fn scalar_chip(&self) -> &Self::ScalarChip;

    /// Assign a elliptic curve point constant.
    fn assign_constant(&self, ctx: &mut Self::Context, ec_point: C) -> Self::AssignedEcPoint;

    /// Assign a elliptic curve point witness.
    fn assign_point(&self, ctx: &mut Self::Context, ec_point: C) -> Self::AssignedEcPoint;

    /// Sum elliptic curve points and constant.
    fn sum_with_const(
        &self,
        ctx: &mut Self::Context,
        values: &[impl Deref<Target = Self::AssignedEcPoint>],
        constant: C,
    ) -> Self::AssignedEcPoint;

    /// Perform fixed base multi-scalar multiplication.
    fn fixed_base_msm(
        &mut self,
        ctx: &mut Self::Context,
        pairs: &[(impl Deref<Target = Self::AssignedScalar>, C)],
    ) -> Self::AssignedEcPoint;

    /// Perform variable base multi-scalar multiplication.
    fn variable_base_msm(
        &mut self,
        ctx: &mut Self::Context,
        pairs: &[(
            impl Deref<Target = Self::AssignedScalar>,
            impl Deref<Target = Self::AssignedEcPoint>,
        )],
    ) -> Self::AssignedEcPoint;

    /// Enforce `lhs` and `rhs` are equal.
    fn assert_equal(
        &self,
        ctx: &mut Self::Context,
        lhs: &Self::AssignedEcPoint,
        rhs: &Self::AssignedEcPoint,
    );
}

mod halo2_lib {
    use crate::halo2_proofs::halo2curves::CurveAffineExt;
    use crate::{
        loader::halo2::{EccInstructions, IntegerInstructions},
        util::arithmetic::CurveAffine,
    };
    use halo2_base::{
        self,
        gates::{builder::GateThreadBuilder, GateChip, GateInstructions, RangeInstructions},
        halo2_proofs::halo2curves::group::ff::PrimeField as _,
        AssignedValue,
        QuantumCell::{Constant, Existing},
    };
    use halo2_ecc::bigint::ProperCrtUint;
    use halo2_ecc::{
        ecc::{BaseFieldEccChip, EcPoint},
        fields::{FieldChip, PrimeField},
    };
    use std::ops::Deref;

    type AssignedInteger<C> = ProperCrtUint<<C as CurveAffine>::ScalarExt>;
    type AssignedEcPoint<C> = EcPoint<<C as CurveAffine>::ScalarExt, AssignedInteger<C>>;

    impl<F: PrimeField> IntegerInstructions<F> for GateChip<F> {
        type Context = GateThreadBuilder<F>;
        type AssignedCell = AssignedValue<F>;
        type AssignedInteger = AssignedValue<F>;

        fn assign_integer(&self, ctx: &mut Self::Context, integer: F) -> Self::AssignedInteger {
            ctx.main(0).load_witness(integer)
        }

        fn assign_constant(&self, ctx: &mut Self::Context, integer: F) -> Self::AssignedInteger {
            ctx.main(0).load_constant(integer)
        }

        fn sum_with_coeff_and_const(
            &self,
            ctx: &mut Self::Context,
            values: &[(F::Scalar, impl Deref<Target = Self::AssignedInteger>)],
            constant: F,
        ) -> Self::AssignedInteger {
            let mut a = Vec::with_capacity(values.len() + 1);
            let mut b = Vec::with_capacity(values.len() + 1);
            if constant != F::zero() {
                a.push(Constant(constant));
                b.push(Constant(F::one()));
            }
            a.extend(values.iter().map(|(_, a)| Existing(*a.deref())));
            b.extend(values.iter().map(|(c, _)| Constant(*c)));
            self.inner_product(ctx.main(0), a, b)
        }

        fn sum_products_with_coeff_and_const(
            &self,
            ctx: &mut Self::Context,
            values: &[(
                F::Scalar,
                impl Deref<Target = Self::AssignedInteger>,
                impl Deref<Target = Self::AssignedInteger>,
            )],
            constant: F,
        ) -> Self::AssignedInteger {
            match values.len() {
                0 => ctx.main(0).load_constant(constant),
                _ => self.sum_products_with_coeff_and_var(
                    ctx.main(0),
                    values.iter().map(|(c, a, b)| (*c, Existing(*a.deref()), Existing(*b.deref()))),
                    Constant(constant),
                ),
            }
        }

        fn sub(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedInteger,
            b: &Self::AssignedInteger,
        ) -> Self::AssignedInteger {
            GateInstructions::sub(self, ctx.main(0), Existing(*a), Existing(*b))
        }

        fn neg(&self, ctx: &mut Self::Context, a: &Self::AssignedInteger) -> Self::AssignedInteger {
            GateInstructions::neg(self, ctx.main(0), Existing(*a))
        }

        fn invert(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedInteger,
        ) -> Self::AssignedInteger {
            // make sure scalar != 0
            let is_zero = self.is_zero(ctx.main(0), *a);
            self.assert_is_const(ctx.main(0), &is_zero, &F::zero());
            GateInstructions::div_unsafe(self, ctx.main(0), Constant(F::one()), Existing(*a))
        }

        fn assert_equal(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedInteger,
            b: &Self::AssignedInteger,
        ) {
            ctx.main(0).constrain_equal(a, b);
        }
    }

    impl<'chip, C: CurveAffineExt> EccInstructions<C> for BaseFieldEccChip<'chip, C>
    where
        C::ScalarExt: PrimeField,
        C::Base: PrimeField,
    {
        type Context = GateThreadBuilder<C::Scalar>;
        type ScalarChip = GateChip<C::Scalar>;
        type AssignedCell = AssignedValue<C::Scalar>;
        type AssignedScalar = AssignedValue<C::Scalar>;
        type AssignedEcPoint = AssignedEcPoint<C>;

        fn scalar_chip(&self) -> &Self::ScalarChip {
            self.field_chip.range().gate()
        }

        fn assign_constant(&self, ctx: &mut Self::Context, point: C) -> Self::AssignedEcPoint {
            self.assign_constant_point(ctx.main(0), point)
        }

        fn assign_point(&self, ctx: &mut Self::Context, point: C) -> Self::AssignedEcPoint {
            self.assign_point(ctx.main(0), point)
        }

        fn sum_with_const(
            &self,
            ctx: &mut Self::Context,
            values: &[impl Deref<Target = Self::AssignedEcPoint>],
            constant: C,
        ) -> Self::AssignedEcPoint {
            let constant = if bool::from(constant.is_identity()) {
                None
            } else {
                let constant = EccInstructions::assign_constant(self, ctx, constant);
                Some(constant)
            };
            self.sum::<C>(
                ctx.main(0),
                constant.into_iter().chain(values.iter().map(|v| v.deref().clone())),
            )
        }

        fn variable_base_msm(
            &mut self,
            builder: &mut Self::Context,
            pairs: &[(
                impl Deref<Target = Self::AssignedScalar>,
                impl Deref<Target = Self::AssignedEcPoint>,
            )],
        ) -> Self::AssignedEcPoint {
            let (scalars, points): (Vec<_>, Vec<_>) = pairs
                .iter()
                .map(|(scalar, point)| (vec![*scalar.deref()], point.deref().clone()))
                .unzip();
            BaseFieldEccChip::<C>::variable_base_msm::<C>(
                self,
                builder,
                &points,
                scalars,
                C::Scalar::NUM_BITS as usize,
            )
        }

        fn fixed_base_msm(
            &mut self,
            builder: &mut Self::Context,
            pairs: &[(impl Deref<Target = Self::AssignedScalar>, C)],
        ) -> Self::AssignedEcPoint {
            let (scalars, points): (Vec<_>, Vec<_>) = pairs
                .iter()
                .filter_map(|(scalar, point)| {
                    if point.is_identity().into() {
                        None
                    } else {
                        Some((vec![*scalar.deref()], *point))
                    }
                })
                .unzip();
            BaseFieldEccChip::<C>::fixed_base_msm::<C>(
                self,
                builder,
                &points,
                scalars,
                C::Scalar::NUM_BITS as usize,
            )
        }

        fn assert_equal(
            &self,
            ctx: &mut Self::Context,
            a: &Self::AssignedEcPoint,
            b: &Self::AssignedEcPoint,
        ) {
            self.assert_equal(ctx.main(0), a.clone(), b.clone());
        }
    }
}
