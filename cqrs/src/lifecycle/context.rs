use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    cell::RefCell,
    collections::HashMap,
};

use cqrs_core::{EventSink, EventSource, NumberedEvent, SnapshotSink, SnapshotSource};

// TODO: Required for `Borrow`/`AsRef` specialization on `Context` types,
//       because Rust doesn't allow negative trait bounds at the moment,
//       so this is the only way we can express specialization by mutually
//       exclude impls using custom trait.
pub trait BorrowableAsContext {}

impl BorrowableAsContext for () {}

impl<Agg, Err> BorrowableAsContext for (dyn SnapshotSource<Agg, Err = Err> + '_) {}

impl<Agg, Err> BorrowableAsContext for (dyn SnapshotSink<Agg, Err = Err> + '_) {}

impl<Agg, Ev, Err> BorrowableAsContext for (dyn EventSource<Agg, Ev, Err = Err> + '_) {}

impl<Agg, Ev, Mt, Err, Ok> BorrowableAsContext
    for (dyn EventSink<Agg, Ev, Mt, Err = Err, Ok = Ok> + '_)
{
}

pub struct Context<Impl> {
    implementation: Impl,
    buffered_events: RefCell<HashMap<TypeId, Box<dyn Any>>>,
}

impl<Impl> Context<Impl> {
    #[inline]
    pub fn new(implementation: Impl) -> Self {
        Self {
            implementation,
            buffered_events: RefCell::new(HashMap::new()),
        }
    }
}

impl<T, Impl> AsRef<T> for Context<Impl>
where
    T: ?Sized,
    Impl: Borrow<T>,
{
    #[inline]
    fn as_ref(&self) -> &T {
        self.implementation.borrow()
    }
}

pub struct ContextWithMeta<Impl, Mt> {
    ctx: Context<Impl>,
    meta: Mt,
}

impl<Impl, Mt> ContextWithMeta<Impl, Mt> {
    #[inline]
    pub fn new(implementation: Impl, meta: Mt) -> Self {
        Self {
            ctx: Context::new(implementation),
            meta,
        }
    }

    #[inline]
    pub fn meta(&self) -> &Mt {
        &self.meta
    }
}

impl<T, Impl, Mt> AsRef<T> for ContextWithMeta<Impl, Mt>
where
    T: ?Sized,
    Impl: Borrow<T>,
{
    #[inline]
    fn as_ref(&self) -> &T {
        self.ctx.as_ref()
    }
}

pub trait BufferedContext: self::private::Sealed {
    fn buffer_event<Ev: 'static>(&self, ev: NumberedEvent<Ev>);

    fn take_buffered_events<Ev: 'static>(&self) -> Vec<NumberedEvent<Ev>>;
}

/// Preventing users from implementing the [`BufferedContext`] trait.
mod private {
    /// Seal of the [`BufferedContext`](super::BufferedContext) trait.
    pub trait Sealed {}
    impl<Impl> Sealed for super::Context<Impl> {}
    impl<Impl, Mt> Sealed for super::ContextWithMeta<Impl, Mt> {}
    impl<Snp, Ctx> Sealed for super::super::Static<Snp, Ctx> {}
}

impl<Impl> BufferedContext for Context<Impl> {
    #[inline]
    fn buffer_event<Ev: 'static>(&self, ev: NumberedEvent<Ev>) {
        self.buffered_events
            .borrow_mut()
            .entry(TypeId::of::<Ev>())
            .or_insert_with(|| Box::new(<Vec<NumberedEvent<Ev>>>::new()))
            .downcast_mut::<Vec<NumberedEvent<Ev>>>()
            .unwrap()
            .push(ev)
    }

    #[inline]
    fn take_buffered_events<Ev: 'static>(&self) -> Vec<NumberedEvent<Ev>> {
        let events = self
            .buffered_events
            .borrow_mut()
            .remove(&TypeId::of::<Ev>());
        if events.is_none() {
            return vec![];
        }
        *events
            .unwrap()
            .downcast::<Vec<NumberedEvent<Ev>>>()
            .unwrap()
    }
}

impl<Impl, Mt> BufferedContext for ContextWithMeta<Impl, Mt> {
    #[inline]
    fn buffer_event<Ev: 'static>(&self, ev: NumberedEvent<Ev>) {
        self.ctx.buffer_event(ev)
    }

    #[inline]
    fn take_buffered_events<Ev: 'static>(&self) -> Vec<NumberedEvent<Ev>> {
        self.ctx.take_buffered_events()
    }
}
