use std::borrow::Borrow;

use cqrs_core::{
    Aggregate, AlwaysSnapshot, Command, CommandHandler, Event, EventSink, EventSource,
    EventSourced, HydratedAggregate, IntoEvents, IntoTryStream, NeverSnapshot,
    SnapshotRecommendation, SnapshotSink, SnapshotSource, SnapshotStrategy,
};
use futures::{future, TryStreamExt as _};

/// TODO
#[derive(Debug)]
pub struct System<Snp, Ctx> {
    snapshot_strategy: Snp,
    ctx: Ctx,
}

impl<Snp, Ctx> System<Snp, Ctx> {
    /// TODO
    #[inline]
    pub fn new(strategy: Snp, ctx: Ctx) -> Self {
        Self {
            snapshot_strategy: strategy,
            ctx,
        }
    }
}

impl<Ctx> From<Ctx> for System<AlwaysSnapshot, Ctx> {
    #[inline]
    fn from(ctx: Ctx) -> Self {
        Self {
            snapshot_strategy: AlwaysSnapshot,
            ctx,
        }
    }
}

impl<Ctx> From<Ctx> for System<NeverSnapshot, Ctx> {
    #[inline]
    fn from(ctx: Ctx) -> Self {
        Self {
            snapshot_strategy: NeverSnapshot,
            ctx,
        }
    }
}

impl<Snp, Ctx> System<Snp, Ctx> {
    /// TODO
    pub async fn load_aggregate_from_snapshot<SsSrc, A>(
        &self,
        id: &A::Id,
    ) -> Result<Option<HydratedAggregate<A>>, SsSrc::Err>
    where
        A: Aggregate,
        Ctx: Borrow<SsSrc>,
        SsSrc: SnapshotSource<A>,
    {
        let snapshot_source: &SsSrc = self.ctx.borrow();
        let agg = snapshot_source
            .load_snapshot(id)
            .await?
            .map(|(agg, ver)| HydratedAggregate::from_snapshot(agg, ver));
        Ok(agg)
    }

    /// TODO
    pub async fn rehydrate_aggregate<EvSrc, E, A>(
        &self,
        agg: &mut HydratedAggregate<A>,
    ) -> Result<(), EvSrc::Err>
    where
        A: Aggregate + EventSourced<E>,
        E: Event,
        Ctx: Borrow<EvSrc>,
        EvSrc: EventSource<A, E>,
    {
        let events_source: &EvSrc = self.ctx.borrow();
        events_source
            .read_events(agg.id(), agg.version().into())
            .into_try_stream()
            .try_for_each(|ev| future::ok(agg.apply(&ev)))
            .await
    }

    /// TODO
    pub async fn load_aggregate_and_rehydrate<SsSrc, EvSrc, E, A>(
        &self,
        id: &A::Id,
    ) -> Result<Option<HydratedAggregate<A>>, LoadError<SsSrc::Err, EvSrc::Err>>
    where
        A: Aggregate + EventSourced<E>,
        E: Event,
        Ctx: Borrow<SsSrc> + Borrow<EvSrc>,
        SsSrc: SnapshotSource<A>,
        EvSrc: EventSource<A, E>,
    {
        let agg = self
            .load_aggregate_from_snapshot::<SsSrc, _>(id)
            .await
            .map_err(LoadError::Snapshot)?;
        if agg.is_none() {
            return Ok(None);
        }

        let mut agg = agg.unwrap();
        self.rehydrate_aggregate::<EvSrc, _, _>(&mut agg)
            .await
            .map_err(LoadError::Events)?;
        Ok(Some(agg))
    }
}

/// TODO
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LoadError<S, E> {
    /// TODO
    Snapshot(S),
    /// TODO
    Events(E),
}

type CommandHandlerErr<C> = <<C as Command>::Aggregate as CommandHandler<C>>::Err;
type CommandHandlerEvent<C> = <<C as Command>::Aggregate as CommandHandler<C>>::Event;
type CommandHandlerContext<C> = <<C as Command>::Aggregate as CommandHandler<C>>::Context;

impl<Snp, Ctx> System<Snp, Ctx>
where
    Snp: SnapshotStrategy,
{
    /// TODO
    pub async fn apply_events_and_persist<EvSnk, SsSnk, E, A, Es, M>(
        &self,
        agg: &mut HydratedAggregate<A>,
        events: Es,
        meta: M,
    ) -> Result<(), PersistError<EvSnk::Err, SsSnk::Err>>
    where
        A: Aggregate + EventSourced<E>,
        E: Event,
        Es: AsRef<[E]>,
        Ctx: Borrow<EvSnk> + Borrow<SsSnk>,
        EvSnk: EventSink<A, E, M>,
        SsSnk: SnapshotSink<A>,
    {
        let event_sink: &EvSnk = self.ctx.borrow();
        let events = event_sink
            .append_events(agg.state().id(), events.as_ref(), meta)
            .await
            .map_err(PersistError::Events)?;

        for ev in events {
            agg.apply(&ev)
        }

        let rcmnd = self
            .snapshot_strategy
            .snapshot_recommendation(agg.version(), agg.snapshot_version());
        if let SnapshotRecommendation::ShouldSnapshot = rcmnd {
            let shapshot_sink: &SsSnk = self.ctx.borrow();
            shapshot_sink
                .persist_snapshot(agg.state(), agg.version())
                .await
                .map_err(PersistError::Snapshot)?;

            agg.set_snapshot_version(agg.version())
        }

        Ok(())
    }

    /// TODO
    pub async fn exec_command_and_persist<EvSnk, SsSnk, C, M>(
        &self,
        cmd: C,
        agg: Option<HydratedAggregate<C::Aggregate>>,
        meta: M,
    ) -> Result<
        HydratedAggregate<C::Aggregate>,
        ExecAndPersistError<C::Aggregate, CommandHandlerErr<C>, EvSnk::Err, SsSnk::Err>,
    >
    where
        C: Command,
        C::Aggregate: CommandHandler<C> + EventSourced<CommandHandlerEvent<C>>,
        Ctx: Borrow<CommandHandlerContext<C>> + Borrow<EvSnk> + Borrow<SsSnk>,
        EvSnk: EventSink<C::Aggregate, CommandHandlerEvent<C>, M>,
        SsSnk: SnapshotSink<C::Aggregate>,
    {
        let mut agg = agg.unwrap_or_default();
        match agg.state().handle_command(cmd, self.ctx.borrow()).await {
            Ok(ev) => {
                self.apply_events_and_persist::<EvSnk, SsSnk, _, _, _, _>(
                    &mut agg,
                    ev.into_events(),
                    meta,
                )
                .await?;
                Ok(agg)
            }
            Err(e) => Err(ExecAndPersistError::Exec(agg, e)),
        }
    }

    /// TODO
    pub async fn load_exec_and_persist<SsSrc, EvSrc, EvSnk, SsSnk, C, M>(
        &self,
        cmd: C,
        meta: M,
    ) -> Result<
        Option<HydratedAggregate<C::Aggregate>>,
        LoadExecAndPersistError<
            C::Aggregate,
            CommandHandlerErr<C>,
            SsSrc::Err,
            EvSrc::Err,
            EvSnk::Err,
            SsSnk::Err,
        >,
    >
    where
        C: Command,
        C::Aggregate: CommandHandler<C> + EventSourced<CommandHandlerEvent<C>>,
        Ctx: Borrow<CommandHandlerContext<C>>
            + Borrow<SsSrc>
            + Borrow<EvSrc>
            + Borrow<EvSnk>
            + Borrow<SsSnk>,
        SsSrc: SnapshotSource<C::Aggregate>,
        EvSrc: EventSource<C::Aggregate, CommandHandlerEvent<C>>,
        EvSnk: EventSink<C::Aggregate, CommandHandlerEvent<C>, M>,
        SsSnk: SnapshotSink<C::Aggregate>,
    {
        let id = cmd.aggregate_id().unwrap(); // TODO: what is None??

        let agg = self
            .load_aggregate_and_rehydrate::<SsSrc, EvSrc, _, _>(id)
            .await?;
        // TODO: what if None??
        if agg.is_none() {
            return Ok(None);
        }

        let agg = self
            .exec_command_and_persist::<EvSnk, SsSnk, _, _>(cmd, Some(agg.unwrap()), meta)
            .await?;
        Ok(Some(agg))
    }
}

/// TODO
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistError<E, S> {
    /// TODO
    Events(E),
    /// TODO
    Snapshot(S),
}

/// TODO
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecAndPersistError<A, E, EvSnkErr, SsSnkErr> {
    /// TODO
    Exec(HydratedAggregate<A>, E),
    /// TODO
    Persist(PersistError<EvSnkErr, SsSnkErr>),
}

impl<A, E, EvSnkErr, SsSnkErr> From<PersistError<EvSnkErr, SsSnkErr>>
    for ExecAndPersistError<A, E, EvSnkErr, SsSnkErr>
{
    #[inline]
    fn from(err: PersistError<EvSnkErr, SsSnkErr>) -> Self {
        ExecAndPersistError::Persist(err)
    }
}

/// TODO
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadExecAndPersistError<A, E, SsSrcErr, EvSrcErr, EvSnkErr, SsSnkErr> {
    /// TODO
    Load(LoadError<SsSrcErr, EvSrcErr>),
    /// TODO
    Exec(HydratedAggregate<A>, E),
    /// TODO
    Persist(PersistError<EvSnkErr, SsSnkErr>),
}

impl<A, E, SsSrcErr, EvSrcErr, EvSnkErr, SsSnkErr> From<PersistError<EvSnkErr, SsSnkErr>>
    for LoadExecAndPersistError<A, E, SsSrcErr, EvSrcErr, EvSnkErr, SsSnkErr>
{
    #[inline]
    fn from(err: PersistError<EvSnkErr, SsSnkErr>) -> Self {
        LoadExecAndPersistError::Persist(err)
    }
}

impl<A, E, SsSrcErr, EvSrcErr, EvSnkErr, SsSnkErr> From<LoadError<SsSrcErr, EvSrcErr>>
    for LoadExecAndPersistError<A, E, SsSrcErr, EvSrcErr, EvSnkErr, SsSnkErr>
{
    #[inline]
    fn from(err: LoadError<SsSrcErr, EvSrcErr>) -> Self {
        LoadExecAndPersistError::Load(err)
    }
}

impl<A, E, SsSrcErr, EvSrcErr, EvSnkErr, SsSnkErr>
    From<ExecAndPersistError<A, E, EvSnkErr, SsSnkErr>>
    for LoadExecAndPersistError<A, E, SsSrcErr, EvSrcErr, EvSnkErr, SsSnkErr>
{
    #[inline]
    fn from(err: ExecAndPersistError<A, E, EvSnkErr, SsSnkErr>) -> Self {
        match err {
            ExecAndPersistError::Exec(agg, e) => LoadExecAndPersistError::Exec(agg, e),
            ExecAndPersistError::Persist(e) => LoadExecAndPersistError::Persist(e),
        }
    }
}
