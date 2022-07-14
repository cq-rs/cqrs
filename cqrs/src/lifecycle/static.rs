use std::borrow::Borrow;

use async_trait::async_trait;
use cqrs_core::{
    Aggregate, Command, CommandHandler, Event, EventSink, EventSource, EventSourced,
    HydratedAggregate, IntoEvents, SnapshotSink, SnapshotSource, SnapshotStrategy,
};

use crate::{CommandBus, EventHandler, EventProcessingConfiguration, RegisteredEvent};

use super::{
    Basic, BorrowableAsContext, BufferedContext, CommandHandlerContext, CommandHandlerErr,
    CommandHandlerEvent, CommandHandlerOk, Context, ContextWithMeta, EventSinkErr, EventSourceErr,
    ExecAndPersistError, LoadError, LoadExecAndPersistError, LoadRehydrateAndPersistError,
    PersistError, SnapshotSinkErr, SnapshotSourceErr,
};

#[derive(Debug)]
pub struct Static<Snp, Ctx> {
    basic_lifecycle: Basic<Snp>,
    ctx: Ctx,
}

impl<Snp, Ctx> Static<Snp, Ctx> {
    #[inline]
    pub fn new(snapshot_strategy: Snp, ctx: Ctx) -> Self {
        Self {
            basic_lifecycle: Basic::new(snapshot_strategy),
            ctx,
        }
    }
}

impl<Snp, Ctx> AsRef<Static<Snp, Ctx>> for Static<Snp, Ctx> {
    #[inline(always)]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<T, Snp, Ctx> AsRef<T> for Static<Snp, Ctx>
where
    T: BorrowableAsContext + ?Sized,
    Ctx: AsRef<T>,
{
    #[inline]
    fn as_ref(&self) -> &T {
        self.ctx.as_ref()
    }
}

impl<Snp, Ctx> Static<Snp, Ctx> {
    #[inline]
    pub async fn load_aggregate_from_snapshot<SsSrc, Agg>(
        &self,
        id: &Agg::Id,
    ) -> Result<Option<HydratedAggregate<Agg>>, SsSrc::Err>
    where
        Agg: Aggregate,
        SsSrc: SnapshotSource<Agg> + ?Sized,
        Ctx: AsRef<SsSrc>,
    {
        self.basic_lifecycle
            .load_aggregate_from_snapshot::<SsSrc, _>(id, self.ctx.as_ref())
            .await
    }

    #[inline]
    pub async fn load_aggregates_from_snapshot<SsSrc, Agg>(
        &self,
        ids: &[Agg::Id],
    ) -> Result<Vec<HydratedAggregate<Agg>>, SsSrc::Err>
    where
        Agg: Aggregate,
        SsSrc: SnapshotSource<Agg> + ?Sized,
        Ctx: AsRef<SsSrc>,
    {
        self.basic_lifecycle
            .load_aggregates_from_snapshot::<SsSrc, _>(ids, self.ctx.as_ref())
            .await
    }

    #[inline]
    pub async fn rehydrate_aggregate<EvSrc, Ev, Agg>(
        &self,
        agg: &mut HydratedAggregate<Agg>,
    ) -> Result<(), EvSrc::Err>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event,
        EvSrc: EventSource<Agg, Ev> + ?Sized,
        Ctx: AsRef<EvSrc>,
    {
        self.basic_lifecycle
            .rehydrate_aggregate::<EvSrc, Ev, _>(agg, self.ctx.as_ref())
            .await
    }

    #[inline]
    pub async fn load_aggregate_and_rehydrate<SsSrc, EvSrc, Ev, Agg>(
        &self,
        id: &Agg::Id,
    ) -> Result<Option<HydratedAggregate<Agg>>, LoadError<SsSrc::Err, EvSrc::Err>>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event,
        SsSrc: SnapshotSource<Agg> + ?Sized,
        EvSrc: EventSource<Agg, Ev> + ?Sized,
        Ctx: AsRef<SsSrc> + AsRef<EvSrc>,
    {
        self.basic_lifecycle
            .load_aggregate_and_rehydrate::<SsSrc, EvSrc, Ev, _, _>(id, &self.ctx)
            .await
    }

    #[inline]
    pub async fn load_aggregates_and_rehydrate<SsSrc, EvSrc, Ev, Agg>(
        &self,
        ids: &[Agg::Id],
    ) -> Result<Vec<HydratedAggregate<Agg>>, LoadError<SsSrc::Err, EvSrc::Err>>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event,
        SsSrc: SnapshotSource<Agg> + ?Sized,
        EvSrc: EventSource<Agg, Ev> + ?Sized,
        Ctx: AsRef<SsSrc> + AsRef<EvSrc>,
    {
        self.basic_lifecycle
            .load_aggregates_and_rehydrate::<SsSrc, EvSrc, Ev, _, _>(ids, &self.ctx)
            .await
    }
}

impl<Snp, Ctx> Static<Snp, Ctx>
where
    Snp: SnapshotStrategy,
{
    #[inline]
    pub async fn persist_aggregate<SsSnk, Agg>(
        &self,
        agg: &mut HydratedAggregate<Agg>,
    ) -> Result<(), SsSnk::Err>
    where
        Agg: Aggregate,
        SsSnk: SnapshotSink<Agg> + ?Sized,
        Ctx: AsRef<SsSnk>,
    {
        self.basic_lifecycle
            .persist_aggregate::<SsSnk, _, _>(agg, &self.ctx)
            .await
    }

    #[inline]
    pub async fn persist_aggregates<SsSnk, Agg>(
        &self,
        aggs: &mut [HydratedAggregate<Agg>],
    ) -> Result<(), SsSnk::Err>
    where
        Agg: Aggregate,
        SsSnk: SnapshotSink<Agg> + ?Sized,
        Ctx: AsRef<SsSnk>,
    {
        self.basic_lifecycle
            .persist_aggregates::<SsSnk, _, _>(aggs, &self.ctx)
            .await
    }

    #[inline]
    pub async fn load_aggregate_rehydrate_and_persist<SsSrc, EvSrc, SsSnk, Ev, Agg>(
        &self,
        id: &Agg::Id,
    ) -> Result<(), LoadRehydrateAndPersistError<SsSrc::Err, EvSrc::Err, SsSnk::Err>>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event,
        SsSrc: SnapshotSource<Agg> + ?Sized,
        EvSrc: EventSource<Agg, Ev> + ?Sized,
        SsSnk: SnapshotSink<Agg> + ?Sized,
        Ctx: AsRef<SsSrc> + AsRef<EvSrc> + AsRef<SsSnk>,
    {
        self.basic_lifecycle
            .load_aggregate_rehydrate_and_persist::<SsSrc, EvSrc, SsSnk, Ev, _, _>(id, &self.ctx)
            .await
    }

    #[inline]
    pub async fn load_aggregates_rehydrate_and_persist<SsSrc, EvSrc, SsSnk, Ev, Agg>(
        &self,
        ids: &[Agg::Id],
    ) -> Result<(), LoadRehydrateAndPersistError<SsSrc::Err, EvSrc::Err, SsSnk::Err>>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event,
        SsSrc: SnapshotSource<Agg> + ?Sized,
        EvSrc: EventSource<Agg, Ev> + ?Sized,
        SsSnk: SnapshotSink<Agg> + ?Sized,
        Ctx: AsRef<SsSrc> + AsRef<EvSrc> + AsRef<SsSnk>,
    {
        self.basic_lifecycle
            .load_aggregates_rehydrate_and_persist::<SsSrc, EvSrc, SsSnk, Ev, _, _>(ids, &self.ctx)
            .await
    }
}

impl<Snp, Impl> Static<Snp, Context<Impl>>
where
    Snp: SnapshotStrategy,
{
    #[inline]
    pub async fn apply_events_and_persist<EvSnk, SsSnk, Ev, Agg, Evs, Mt>(
        &self,
        agg: &mut HydratedAggregate<Agg>,
        events: Evs,
        meta: &Mt,
    ) -> Result<(), PersistError<EvSnk::Err, SsSnk::Err>>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event + 'static,
        Evs: AsRef<[Ev]>,
        Mt: ?Sized,
        EvSnk: EventSink<Agg, Ev, Mt> + ?Sized,
        SsSnk: SnapshotSink<Agg> + ?Sized,
        Impl: Borrow<EvSnk> + Borrow<SsSnk>,
    {
        self.basic_lifecycle
            .apply_events_and_persist::<EvSnk, SsSnk, Ev, _, _, _, _, _>(
                agg,
                events,
                meta,
                &self.ctx,
                Some(&self.ctx),
            )
            .await
    }

    #[inline]
    pub async fn exec_command_and_persist<EvSnk, SsSnk, Cmd, Mt>(
        &self,
        cmd: Cmd,
        agg: Option<HydratedAggregate<Cmd::Aggregate>>,
        meta: &Mt,
    ) -> Result<
        HydratedAggregate<Cmd::Aggregate>,
        ExecAndPersistError<Cmd::Aggregate, CommandHandlerErr<Cmd>, EvSnk::Err, SsSnk::Err>,
    >
    where
        Cmd: Command,
        Cmd::Aggregate: CommandHandler<Cmd> + EventSourced<CommandHandlerEvent<Cmd>>,
        CommandHandlerEvent<Cmd>: Event + 'static,
        CommandHandlerOk<Cmd>: IntoEvents<CommandHandlerEvent<Cmd>> + 'static,
        Mt: ?Sized,
        EvSnk: EventSink<Cmd::Aggregate, CommandHandlerEvent<Cmd>, Mt> + ?Sized,
        SsSnk: SnapshotSink<Cmd::Aggregate> + ?Sized,
        Impl: Borrow<EvSnk> + Borrow<SsSnk>,
        Self: AsRef<CommandHandlerContext<Cmd>>,
    {
        self.basic_lifecycle
            .exec_command_and_persist::<EvSnk, SsSnk, _, _, _, _>(
                cmd,
                agg,
                meta,
                self.as_ref(),
                &self.ctx,
                Some(&self.ctx),
            )
            .await
    }

    #[inline]
    pub async fn load_aggregate_exec_command_and_persist<SsSrc, EvSrc, EvSnk, SsSnk, Cmd, Mt>(
        &self,
        cmd: Cmd,
        meta: &Mt,
    ) -> Result<
        Option<HydratedAggregate<Cmd::Aggregate>>,
        LoadExecAndPersistError<
            Cmd::Aggregate,
            CommandHandlerErr<Cmd>,
            SsSrc::Err,
            EvSrc::Err,
            EvSnk::Err,
            SsSnk::Err,
        >,
    >
    where
        Cmd: Command,
        Cmd::Aggregate: CommandHandler<Cmd> + EventSourced<CommandHandlerEvent<Cmd>>,
        CommandHandlerEvent<Cmd>: Event + 'static,
        CommandHandlerOk<Cmd>: IntoEvents<CommandHandlerEvent<Cmd>> + 'static,
        Mt: ?Sized,
        SsSrc: SnapshotSource<Cmd::Aggregate> + ?Sized,
        EvSrc: EventSource<Cmd::Aggregate, CommandHandlerEvent<Cmd>> + ?Sized,
        EvSnk: EventSink<Cmd::Aggregate, CommandHandlerEvent<Cmd>, Mt> + ?Sized,
        SsSnk: SnapshotSink<Cmd::Aggregate> + ?Sized,
        Impl: Borrow<SsSrc> + Borrow<EvSrc> + Borrow<EvSnk> + Borrow<SsSnk>,
        Self: AsRef<CommandHandlerContext<Cmd>>,
    {
        self.basic_lifecycle
            .load_aggregate_exec_command_and_persist::<SsSrc, EvSrc, EvSnk, SsSnk, _, _, _, _>(
                cmd,
                meta,
                self.as_ref(),
                &self.ctx,
                Some(&self.ctx),
            )
            .await
    }
}

impl<Snp, Impl, Mt> Static<Snp, ContextWithMeta<Impl, Mt>>
where
    Snp: SnapshotStrategy,
{
    #[inline]
    pub async fn apply_events_and_persist<EvSnk, SsSnk, Ev, Agg, Evs>(
        &self,
        agg: &mut HydratedAggregate<Agg>,
        events: Evs,
    ) -> Result<(), PersistError<EvSnk::Err, SsSnk::Err>>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event + 'static,
        Evs: AsRef<[Ev]>,
        EvSnk: EventSink<Agg, Ev, Mt> + ?Sized,
        SsSnk: SnapshotSink<Agg> + ?Sized,
        Impl: Borrow<EvSnk> + Borrow<SsSnk>,
    {
        self.basic_lifecycle
            .apply_events_and_persist::<EvSnk, SsSnk, Ev, _, _, _, _, _>(
                agg,
                events,
                self.ctx.meta(),
                &self.ctx,
                Some(&self.ctx),
            )
            .await
    }

    #[inline]
    pub async fn exec_command_and_persist<EvSnk, SsSnk, Cmd>(
        &self,
        cmd: Cmd,
        agg: Option<HydratedAggregate<Cmd::Aggregate>>,
    ) -> Result<
        HydratedAggregate<Cmd::Aggregate>,
        ExecAndPersistError<Cmd::Aggregate, CommandHandlerErr<Cmd>, EvSnk::Err, SsSnk::Err>,
    >
    where
        Cmd: Command,
        Cmd::Aggregate: CommandHandler<Cmd> + EventSourced<CommandHandlerEvent<Cmd>>,
        CommandHandlerEvent<Cmd>: Event + 'static,
        CommandHandlerOk<Cmd>: IntoEvents<CommandHandlerEvent<Cmd>> + 'static,
        EvSnk: EventSink<Cmd::Aggregate, CommandHandlerEvent<Cmd>, Mt> + ?Sized,
        SsSnk: SnapshotSink<Cmd::Aggregate> + ?Sized,
        Impl: Borrow<EvSnk> + Borrow<SsSnk>,
        Self: AsRef<CommandHandlerContext<Cmd>>,
    {
        self.basic_lifecycle
            .exec_command_and_persist::<EvSnk, SsSnk, _, _, _, _>(
                cmd,
                agg,
                self.ctx.meta(),
                self.as_ref(),
                &self.ctx,
                Some(&self.ctx),
            )
            .await
    }

    #[inline]
    pub async fn load_aggregate_exec_command_and_persist<SsSrc, EvSrc, EvSnk, SsSnk, Cmd>(
        &self,
        cmd: Cmd,
    ) -> Result<
        Option<HydratedAggregate<Cmd::Aggregate>>,
        LoadExecAndPersistError<
            Cmd::Aggregate,
            CommandHandlerErr<Cmd>,
            SsSrc::Err,
            EvSrc::Err,
            EvSnk::Err,
            SsSnk::Err,
        >,
    >
    where
        Cmd: Command,
        Cmd::Aggregate: CommandHandler<Cmd> + EventSourced<CommandHandlerEvent<Cmd>>,
        CommandHandlerEvent<Cmd>: Event + 'static,
        CommandHandlerOk<Cmd>: IntoEvents<CommandHandlerEvent<Cmd>> + 'static,
        SsSrc: SnapshotSource<Cmd::Aggregate> + ?Sized,
        EvSrc: EventSource<Cmd::Aggregate, CommandHandlerEvent<Cmd>> + ?Sized,
        EvSnk: EventSink<Cmd::Aggregate, CommandHandlerEvent<Cmd>, Mt> + ?Sized,
        SsSnk: SnapshotSink<Cmd::Aggregate> + ?Sized,
        Impl: Borrow<SsSrc> + Borrow<EvSrc> + Borrow<EvSnk> + Borrow<SsSnk>,
        Self: AsRef<CommandHandlerContext<Cmd>>,
    {
        self.basic_lifecycle
            .load_aggregate_exec_command_and_persist::<SsSrc, EvSrc, EvSnk, SsSnk, _, _, _, _>(
                cmd,
                self.ctx.meta(),
                self.as_ref(),
                &self.ctx,
                Some(&self.ctx),
            )
            .await
    }
}

impl<Snp, Ctx> Static<Snp, Ctx> {
    pub async fn exec_event_handlers<Ev, Err>(
        &self,
        cfg: &EventProcessingConfiguration,
    ) -> Result<(), Err>
    where
        Ev: RegisteredEvent,
        Snp: 'static,
        Ctx: BufferedContext + 'static,
        Err: 'static,
    {
        Ok(for ev in self.ctx.take_buffered_events::<Ev>() {
            // TODO: execute handlers concurrently?
            for handler in cfg.iter_event_handlers_of::<Ev, Self, Err>(&ev.data) {
                handler.on(&ev.data, &self).await?
            }
        })
    }
}

#[async_trait(?Send)]
impl<Snp, Impl, Mt, Cmd> CommandBus<Cmd> for Static<Snp, ContextWithMeta<Impl, Mt>>
where
    Snp: SnapshotStrategy,
    Cmd: Command,
    Cmd::Aggregate: CommandHandler<Cmd> + EventSourced<CommandHandlerEvent<Cmd>>,
    CommandHandlerEvent<Cmd>: Event + 'static,
    CommandHandlerOk<Cmd>: IntoEvents<CommandHandlerEvent<Cmd>> + 'static,
    Impl: SnapshotSource<Cmd::Aggregate>
        + EventSource<Cmd::Aggregate, CommandHandlerEvent<Cmd>>
        + EventSink<Cmd::Aggregate, CommandHandlerEvent<Cmd>, Mt>
        + SnapshotSink<Cmd::Aggregate>,
    Self: AsRef<CommandHandlerContext<Cmd>>,
{
    type Err = LoadExecAndPersistError<
        Cmd::Aggregate,
        CommandHandlerErr<Cmd>,
        SnapshotSourceErr<Impl, Cmd>,
        EventSourceErr<Impl, Cmd>,
        EventSinkErr<Impl, Cmd, Mt>,
        SnapshotSinkErr<Impl, Cmd>,
    >;
    type Ok = Option<HydratedAggregate<Cmd::Aggregate>>;

    #[inline]
    async fn dispatch(&self, cmd: Cmd) -> Result<Self::Ok, Self::Err>
    where
        Cmd: 'async_trait,
    {
        self.load_aggregate_exec_command_and_persist::<Impl, Impl, Impl, Impl, _>(cmd)
            .await
    }
}

type DynCommandBus<'a, Cmd, Impl, Mt> = (dyn CommandBus<
    Cmd,
    Ok = Option<HydratedAggregate<<Cmd as Command>::Aggregate>>,
    Err = LoadExecAndPersistError<
        <Cmd as Command>::Aggregate,
        CommandHandlerErr<Cmd>,
        SnapshotSourceErr<Impl, Cmd>,
        EventSourceErr<Impl, Cmd>,
        EventSinkErr<Impl, Cmd, Mt>,
        SnapshotSinkErr<Impl, Cmd>,
    >,
> + 'a);

impl<'a, Snp, Impl, Mt, Cmd> AsRef<DynCommandBus<'a, Cmd, Impl, Mt>>
    for Static<Snp, ContextWithMeta<Impl, Mt>>
where
    Snp: SnapshotStrategy + 'a,
    Mt: 'a,
    Cmd: Command,
    Cmd::Aggregate: CommandHandler<Cmd> + EventSourced<CommandHandlerEvent<Cmd>>,
    CommandHandlerEvent<Cmd>: Event + 'static,
    CommandHandlerOk<Cmd>: IntoEvents<CommandHandlerEvent<Cmd>> + 'static,
    Impl: SnapshotSource<Cmd::Aggregate>
        + EventSource<Cmd::Aggregate, CommandHandlerEvent<Cmd>>
        + EventSink<Cmd::Aggregate, CommandHandlerEvent<Cmd>, Mt>
        + SnapshotSink<Cmd::Aggregate>
        + 'a,
    Self: AsRef<CommandHandlerContext<Cmd>>,
{
    #[inline]
    fn as_ref(&self) -> &DynCommandBus<'a, Cmd, Impl, Mt> {
        self
    }
}
