use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    convert::TryFrom,
    fmt,
    marker::PhantomData,
    sync::{atomic::AtomicPtr, Arc},
};

use async_trait::async_trait;
use cqrs_core::Event;

#[async_trait(?Send)]
pub trait EventHandler<Ev: Event + ?Sized> {
    type Context: ?Sized;
    type Err;

    // TODO: meta?
    async fn on(&self, event: &Ev, ctx: &Self::Context) -> Result<(), Self::Err>;
}

// TODO: Implement `EventHandler` for:
//       async fn(&Ev, &Ctx) -> Result<(), Err>
//       async fn(&Ev) -> Result<(), Err>
//       async fn(&Ev, &Ctx)
//       async fn(&Ev)
//       Currently, I haven't figured out how to do this event with wrapper
//       types, because `async_trait` requires using HRTB to omit lifetime
//       issues, while having HRTB in type signature doesn't allow to accept
//       `async fn` pointers as type mismatches.
//       See: https://users.rust-lang.org/t/33006
/*
struct EventHandlerTryFn<'a, 'b, F, Fut, Ev, Ctx, Err>(
    F,
    PhantomData<Fut>,
    PhantomData<&'a Ev>,
    PhantomData<&'b Ctx>,
    PhantomData<Err>,
)
where
    F: Fn(&'a Ev, &'b Ctx) -> Fut,
    Fut: Future<Output = Result<(), Err>>,
    Ev: ?Sized + 'a,
    Ctx: ?Sized + 'b;

impl<'a, 'b, F, Fut, Ev, Ctx, Err> EventHandlerTryFn<'a, 'b, F, Fut, Ev, Ctx, Err>
where
    F: Fn(&'a Ev, &'b Ctx) -> Fut,
    Fut: Future<Output = Result<(), Err>>,
    Ev: ?Sized + 'a,
    Ctx: ?Sized + 'b,
{
    fn call<'e: 'r + 'a, 'c: 'r + 'b, 'r>(&self, ev: &'e Ev, ctx: &'c Ctx) -> Fut
    where Fut: 'r
    {
        self.0(ev, ctx)
    }
}

#[async_trait(?Send)]
impl<'a, 'b, F, Fut, Ev, Ctx, Err> EventHandler<Ev> for EventHandlerTryFn<'a, 'b, F, Fut, Ev, Ctx, Err>
where
    F: Fn(&'a Ev, &'b Ctx) -> Fut,
    Fut: Future<Output = Result<(), Err>>,
    Ev: Event + ?Sized + 'a,
    Ctx: ?Sized + 'b,
    Err: 'static,
{
    type Context = Ctx;
    type Err = Err;

    #[inline]
    async fn on(&self, event: &Ev, ctx: &Self::Context) -> Result<(), Self::Err> {
        self.call(event, ctx).await
    }
}

async fn some<Ev: Event + ?Sized>(ev: &Ev, ctx: &()) -> Result<(), std::convert::Infallible> {
    Ok(())
}

fn test_some() {
    assert_is_event_handler(EventHandlerTryFn(
        some,
        PhantomData,
        PhantomData,
        PhantomData,
        PhantomData,
    ))
}

fn assert_is_event_handler<T, Ev>(_: T)
where
    T: EventHandler<Ev>,
    Ev: Event + ?Sized,
{
}
*/

pub trait RegisteredEvent: Event + 'static {
    #[inline]
    fn type_id(&self) -> TypeId;
}

#[derive(Clone, Debug)]
pub struct EventProcessingConfiguration {
    handlers: Arc<EventHandlersRegistry>,
}

sa::assert_impl_all!(EventProcessingConfiguration: Send, Sync);

impl EventProcessingConfiguration {
    #[inline]
    pub fn new() -> EventProcessingConfigurationBuilder {
        EventProcessingConfigurationBuilder {
            handlers: EventHandlersRegistry::default(),
        }
    }

    #[inline]
    pub fn iter_event_handlers_of<Ev, Ctx, Err>(
        &self,
        ev: &Ev,
    ) -> impl Iterator<Item = &DynEventHandler<Ev, Ctx, Err>>
    where
        Ev: RegisteredEvent + ?Sized,
        Ctx: ?Sized + 'static,
        Err: 'static,
    {
        self.handlers.iter::<Ev, Ctx, Err>(ev)
    }
}

#[derive(Debug)]
pub struct EventProcessingConfigurationBuilder {
    handlers: EventHandlersRegistry,
}

impl EventProcessingConfigurationBuilder {
    #[inline]
    pub fn build(self) -> EventProcessingConfiguration {
        EventProcessingConfiguration {
            handlers: Arc::new(self.handlers),
        }
    }

    #[inline]
    pub fn register_event_handler<Ev, AsEv, Ctx, Err, H>(&mut self, handler: H)
    where
        Ev: Event + ?Sized + 'static,
        for<'e> &'e Ev: TryFrom<&'e AsEv>,
        AsEv: Event + ?Sized + 'static,
        Ctx: AsRef<H::Context> + ?Sized + 'static,
        Err: From<H::Err> + 'static,
        H: EventHandler<Ev> + Send + Sync + 'static,
    {
        self.handlers.register::<Ev, AsEv, Ctx, Err, H>(handler)
    }
}

#[derive(Debug, Default)]
struct EventHandlersRegistry(
    HashMap<TypeId, HashMap<(TypeId, TypeId, TypeId), HashMap<TypeId, OpaqueEventHandler>>>,
);

sa::assert_impl_all!(EventHandlersRegistry: Send, Sync);

impl EventHandlersRegistry {
    fn register<Ev, AsEv, Ctx, Err, H>(&mut self, handler: H)
    where
        Ev: Event + ?Sized + 'static,
        for<'e> &'e Ev: TryFrom<&'e AsEv>,
        AsEv: Event + ?Sized + 'static,
        Ctx: AsRef<H::Context> + ?Sized + 'static,
        Err: From<H::Err> + 'static,
        H: EventHandler<Ev> + Send + Sync + 'static,
    {
        let raw =
            RawEventHandler::<H, Ev, Ctx, Err>(handler, PhantomData, PhantomData, PhantomData);
        let r#dyn = DynEventHandler::<AsEv, Ctx, Err>(Box::new(raw));
        let opaque = OpaqueEventHandler(Box::new(r#dyn));
        let _ = self
            .0
            .entry(TypeId::of::<Ev>())
            .or_default()
            .entry((
                TypeId::of::<AsEv>(),
                TypeId::of::<Ctx>(),
                TypeId::of::<Err>(),
            ))
            .or_default()
            .insert(TypeId::of::<H>(), opaque);
    }

    fn iter<Ev, Ctx, Err>(&self, ev: &Ev) -> impl Iterator<Item = &DynEventHandler<Ev, Ctx, Err>>
    where
        Ev: RegisteredEvent + ?Sized,
        Ctx: ?Sized + 'static,
        Err: 'static,
    {
        self.0
            .get(&ev.type_id())
            .map(|v| {
                v.get(&(TypeId::of::<Ev>(), TypeId::of::<Ctx>(), TypeId::of::<Err>()))
                    .map(HashMap::iter)
                    .into_iter()
                    .flatten()
            })
            .into_iter()
            .flatten()
            .map(|(_, boxed_any)| {
                boxed_any
                    .0
                    .as_ref()
                    .downcast_ref::<DynEventHandler<Ev, Ctx, Err>>()
                    .unwrap()
            })
    }
}

struct OpaqueEventHandler(Box<dyn Any + Send + Sync>);

impl fmt::Debug for OpaqueEventHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("OpaqueEventHandler").field(&"..").finish()
    }
}

pub struct DynEventHandler<Ev, Ctx, Err>(
    Box<dyn EventHandler<Ev, Context = Ctx, Err = Err> + Send + Sync>,
)
where
    Ev: ?Sized,
    Ctx: ?Sized;

// `std::env::Args` type is `!Send + !Sync`
sa::assert_impl_all!(DynEventHandler<u8, std::env::Args, std::env::Args>: Send, Sync);

#[async_trait(?Send)]
impl<Ev, Ctx, Err> EventHandler<Ev> for DynEventHandler<Ev, Ctx, Err>
where
    Ev: Event + ?Sized,
    Ctx: ?Sized,
{
    type Context = Ctx;
    type Err = Err;

    #[inline]
    async fn on(&self, event: &Ev, ctx: &Self::Context) -> Result<(), Self::Err> {
        self.0.on(event, ctx).await
    }
}

struct RawEventHandler<H, Ev, Ctx, Err>(
    H,
    PhantomData<AtomicPtr<Box<Ev>>>,
    PhantomData<AtomicPtr<Box<Ctx>>>,
    PhantomData<AtomicPtr<Err>>,
)
where
    Ev: ?Sized,
    Ctx: ?Sized;

// `std::env::Args` type is `!Send + !Sync`
sa::assert_impl_all!(
    RawEventHandler<u8, std::env::Args, std::env::Args, std::env::Args>: Send, Sync
);

#[async_trait(?Send)]
impl<AsEv, H, Ev, Ctx, Err> EventHandler<AsEv> for RawEventHandler<H, Ev, Ctx, Err>
where
    AsEv: Event + ?Sized,
    H: EventHandler<Ev>,
    Ev: Event + ?Sized,
    for<'e> &'e Ev: TryFrom<&'e AsEv>,
    Ctx: AsRef<H::Context> + ?Sized,
    Err: From<H::Err>,
{
    type Context = Ctx;
    type Err = Err;

    #[inline]
    async fn on(&self, event: &AsEv, ctx: &Self::Context) -> Result<(), Self::Err> {
        if let Ok(ev) = <&Ev>::try_from(event) {
            self.0.on(ev, ctx.as_ref()).await.map_err(Err::from)
        } else {
            panic!(
                "Event({}) fails to convert into Event({}) \
                 on calling EventHandler({})",
                type_name::<AsEv>(),
                type_name::<Ev>(),
                type_name::<H>()
            )
        }
    }
}

pub trait EventHandlersRegistrar<Ev, AsEv, Ctx, Err>
where
    Ev: Event + ?Sized + 'static,
    for<'e> &'e Ev: TryFrom<&'e AsEv>,
    AsEv: Event + ?Sized + 'static,
    Ctx: ?Sized + 'static,
    Err: 'static,
{
    fn register_event_handlers(&self, builder: &mut EventProcessingConfigurationBuilder);
}

#[cfg(test)]
mod event_processing_configuration_spec {
    use std::{
        any::TypeId,
        convert::{self, Infallible},
    };

    use async_trait::async_trait;
    use derive_more::{From, TryIntoRef};

    use super::EventProcessingConfiguration;

    struct TestEvent;

    impl crate::Event for TestEvent {
        fn event_type(&self) -> crate::EventType {
            "test"
        }
    }

    impl crate::RegisteredEvent for TestEvent {}

    #[derive(From, TryIntoRef)]
    enum TestAggregateEvent {
        TestEvent(TestEvent),
    }

    impl crate::Event for TestAggregateEvent {
        fn event_type(&self) -> crate::EventType {
            match self {
                Self::TestEvent(e) => e.event_type(),
            }
        }
    }

    impl crate::RegisteredEvent for TestAggregateEvent {
        fn type_id(&self) -> TypeId {
            TypeId::of::<TestEvent>()
        }
    }

    struct TestHandler;

    #[async_trait(?Send)]
    impl crate::EventHandler<TestEvent> for TestHandler {
        type Context = ();
        type Err = Infallible;

        async fn on(&self, ev: &TestEvent, ctx: &Self::Context) -> Result<(), Self::Err> {
            unreachable!()
        }
    }

    struct CustomError;

    impl convert::From<Infallible> for CustomError {
        fn from(e: Infallible) -> Self {
            match e {}
        }
    }

    struct CustomContext(());

    impl AsRef<()> for CustomContext {
        fn as_ref(&self) -> &() {
            &self.0
        }
    }

    #[test]
    fn returns_registered_handlers() {
        let mut cfg = EventProcessingConfiguration::new();
        cfg.register_event_handler::<TestEvent, TestAggregateEvent, CustomContext, CustomError, _>(
            TestHandler,
        );
        let cfg = cfg.build();

        let mut iter = cfg
            .iter_event_handlers_of::<TestAggregateEvent, CustomContext, CustomError>(
                &TestEvent.into(),
            );

        assert!(iter.next().is_some())
    }
}
