use core::marker::PhantomData;

// Tagged scalar helper for Float Out Boy balance-filter intermediates; behavior maps
// live at use sites against `third_party/float-out-boy/src/balance_filter.c:73-134`.
#[repr(transparent)]
pub(crate) struct AxisScalar<Tag>(pub(super) f32, PhantomData<fn() -> Tag>);

impl<Tag> core::fmt::Debug for AxisScalar<Tag> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("AxisScalar").field(&self.0).finish()
    }
}

impl<Tag> Clone for AxisScalar<Tag> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Tag> Copy for AxisScalar<Tag> {}

impl<Tag> PartialEq for AxisScalar<Tag> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<Tag> AxisScalar<Tag> {
    #[inline(always)]
    pub(crate) const fn new(value: f32) -> Self {
        Self(value, PhantomData)
    }
}
