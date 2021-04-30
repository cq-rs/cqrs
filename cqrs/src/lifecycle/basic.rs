use std::{convert, fmt};

use cqrs_core::{
    Aggregate, Command, CommandHandler, Event, EventNumber, EventSink, EventSource, EventSourced,
    HydratedAggregate, IntoEvents as _, NumberedEvent, SnapshotRecommendation, SnapshotSink,
    SnapshotSource, SnapshotStrategy, IntoEvents,
};
use derive_more::{Display, Error, From};
use futures::{future, TryStreamExt as _};
use smallvec::SmallVec;

use super::{BufferedContext, CommandHandlerContext,  CommandHandlerOk,CommandHandlerErr, CommandHandlerEvent};

#[derive(Debug)]
pub struct Basic<Snp> {
    snapshot_strategy: Snp,
}

impl<Snp> Basic<Snp> {
    #[inline]
    pub fn new(snapshot_strategy: Snp) -> Self {
        Self { snapshot_strategy }
    }
}

impl<Snp> AsRef<Basic<Snp>> for Basic<Snp> {
    #[inline(always)]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<Snp> Basic<Snp> {
    pub async fn load_aggregate_from_snapshot<SsSrc, Agg>(
        &self,
        id: &Agg::Id,
        snapshot_source: &SsSrc,
    ) -> Result<Option<HydratedAggregate<Agg>>, SsSrc::Err>
    where
        Agg: Aggregate,
        SsSrc: SnapshotSource<Agg> + ?Sized,
    {
        Ok(snapshot_source
            .load_snapshot(id)
            .await?
            .map(|(agg, ver)| HydratedAggregate::from_snapshot(agg, ver)))
    }

    pub async fn load_aggregates_from_snapshot<SsSrc, Agg>(
        &self,
        ids: &[Agg::Id],
        snapshot_source: &SsSrc,
    ) -> Result<Vec<HydratedAggregate<Agg>>, SsSrc::Err>
    where
        Agg: Aggregate,
        SsSrc: SnapshotSource<Agg> + ?Sized,
    {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        Ok(snapshot_source
            .load_snapshots(ids)
            .await?
            .into_iter()
            .map(|(agg, ver)| HydratedAggregate::from_snapshot(agg, ver))
            .collect())
    }

    pub async fn rehydrate_aggregate<EvSrc, Ev, Agg>(
        &self,
        agg: &mut HydratedAggregate<Agg>,
        event_source: &EvSrc,
    ) -> Result<(), EvSrc::Err>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event,
        EvSrc: EventSource<Agg, Ev> + ?Sized,
    {
        event_source
            .read_events(agg.id(), agg.version().into())
            .try_for_each(|ev| future::ok(agg.apply(&ev)))
            .await
    }

    pub async fn load_aggregate_and_rehydrate<SsSrc, EvSrc, Ev, Agg, Repo>(
        &self,
        id: &Agg::Id,
        repo: &Repo,
    ) -> Result<Option<HydratedAggregate<Agg>>, LoadError<SsSrc::Err, EvSrc::Err>>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event,
        SsSrc: SnapshotSource<Agg> + ?Sized,
        EvSrc: EventSource<Agg, Ev> + ?Sized,
        Repo: AsRef<SsSrc> + AsRef<EvSrc> + ?Sized,
    {
        let agg = self
            .load_aggregate_from_snapshot::<SsSrc, _>(id, repo.as_ref())
            .await
            .map_err(LoadError::Snapshot)?;
        if agg.is_none() {
            return Ok(None);
        }

        let mut agg = agg.unwrap();
        self.rehydrate_aggregate::<EvSrc, Ev, _>(&mut agg, repo.as_ref())
            .await
            .map_err(LoadError::Events)?;
        Ok(Some(agg))
    }

    pub async fn load_aggregates_and_rehydrate<SsSrc, EvSrc, Ev, Agg, Repo>(
        &self,
        ids: &[Agg::Id],
        repo: &Repo,
    ) -> Result<Vec<HydratedAggregate<Agg>>, LoadError<SsSrc::Err, EvSrc::Err>>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event,
        SsSrc: SnapshotSource<Agg> + ?Sized,
        EvSrc: EventSource<Agg, Ev> + ?Sized,
        Repo: AsRef<SsSrc> + AsRef<EvSrc> + ?Sized,
    {
        let mut aggs = self
            .load_aggregates_from_snapshot::<SsSrc, _>(ids, repo.as_ref())
            .await
            .map_err(LoadError::Snapshot)?;
        if aggs.is_empty() {
            return Ok(vec![]);
        }

        // TODO: sequential events loading is inefficient
        for agg in aggs.iter_mut() {
            self.rehydrate_aggregate::<EvSrc, Ev, _>(agg, repo.as_ref())
                .await
                .map_err(LoadError::Events)?;
        }
        Ok(aggs)
    }
}

#[derive(Clone, Copy, Debug, Display, Eq, Error, PartialEq)]
pub enum LoadError<SsSrcErr, EvSrcErr> {
    #[display(fmt = "Loading aggregate snapshot failed: {}", _0)]
    Snapshot(SsSrcErr),
    #[display(fmt = "Loading events failed: {}", _0)]
    Events(EvSrcErr),
}

impl<Snp> Basic<Snp>
where
    Snp: SnapshotStrategy,
{
    pub async fn persist_aggregate<SsSnk, Agg, Repo>(
        &self,
        agg: &mut HydratedAggregate<Agg>,
        repo: &Repo,
    ) -> Result<(), SsSnk::Err>
    where
        Agg: Aggregate,
        SsSnk: SnapshotSink<Agg> + ?Sized,
        Repo: AsRef<SsSnk> + ?Sized,
    {
        let rcmnd = self
            .snapshot_strategy
            .recommendation(agg.version(), agg.snapshot_version());
        if let SnapshotRecommendation::ShouldSnapshot = rcmnd {
            let shapshot_sink: &SsSnk = repo.as_ref();
            shapshot_sink
                .persist_snapshot(agg.state(), agg.version())
                .await?;

            agg.set_snapshot_version(agg.version())
        }
        Ok(())
    }

    pub async fn persist_aggregates<SsSnk, Agg, Repo>(
        &self,
        aggs: &mut [HydratedAggregate<Agg>],
        repo: &Repo,
    ) -> Result<(), SsSnk::Err>
    where
        Agg: Aggregate,
        SsSnk: SnapshotSink<Agg> + ?Sized,
        Repo: AsRef<SsSnk> + ?Sized,
    {
        if aggs.is_empty() {
            return Ok(());
        }

        let should_snapshot_aggs = aggs
            .iter_mut()
            .filter_map(|agg| {
                let rcmnd = self
                    .snapshot_strategy
                    .recommendation(agg.version(), agg.snapshot_version());
                if let SnapshotRecommendation::ShouldSnapshot = rcmnd {
                    Some(agg)
                } else {
                    None
                }
            })
            .collect::<SmallVec<[_; 10]>>();
        if should_snapshot_aggs.is_empty() {
            return Ok(());
        }

        {
            let for_persisting = should_snapshot_aggs
                .iter()
                .map(|agg| (agg.state(), agg.version()))
                .collect::<SmallVec<[_; 10]>>();

            let shapshot_sink: &SsSnk = repo.as_ref();
            shapshot_sink
                .persist_snapshots(for_persisting.as_slice())
                .await?;
        }

        for agg in should_snapshot_aggs.into_iter() {
            agg.set_snapshot_version(agg.version())
        }

        Ok(())
    }

    pub async fn load_aggregate_rehydrate_and_persist<SsSrc, EvSrc, SsSnk, Ev, Agg, Repo>(
        &self,
        id: &Agg::Id,
        repo: &Repo,
    ) -> Result<(), LoadRehydrateAndPersistError<SsSrc::Err, EvSrc::Err, SsSnk::Err>>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event,
        SsSrc: SnapshotSource<Agg> + ?Sized,
        EvSrc: EventSource<Agg, Ev> + ?Sized,
        SsSnk: SnapshotSink<Agg> + ?Sized,
        Repo: AsRef<SsSrc> + AsRef<EvSrc> + AsRef<SsSnk> + ?Sized,
    {
        let mut agg = self
            .load_aggregate_and_rehydrate::<SsSrc, EvSrc, Ev, _, _>(id, repo)
            .await
            .map_err(LoadRehydrateAndPersistError::Load)?;

        if let Some(agg) = agg.as_mut() {
            self.persist_aggregate::<SsSnk, _, _>(agg, repo)
                .await
                .map_err(LoadRehydrateAndPersistError::Persist)?;
        }

        Ok(())
    }

    pub async fn load_aggregates_rehydrate_and_persist<SsSrc, EvSrc, SsSnk, Ev, Agg, Repo>(
        &self,
        ids: &[Agg::Id],
        repo: &Repo,
    ) -> Result<(), LoadRehydrateAndPersistError<SsSrc::Err, EvSrc::Err, SsSnk::Err>>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event,
        SsSrc: SnapshotSource<Agg> + ?Sized,
        EvSrc: EventSource<Agg, Ev> + ?Sized,
        SsSnk: SnapshotSink<Agg> + ?Sized,
        Repo: AsRef<SsSrc> + AsRef<EvSrc> + AsRef<SsSnk> + ?Sized,
    {
        let mut aggs = self
            .load_aggregates_and_rehydrate::<SsSrc, EvSrc, Ev, _, _>(ids, repo)
            .await
            .map_err(LoadRehydrateAndPersistError::Load)?;
        if aggs.is_empty() {
            return Ok(());
        }

        self.persist_aggregates::<SsSnk, _, _>(aggs.as_mut_slice(), repo)
            .await
            .map_err(LoadRehydrateAndPersistError::Persist)
    }

    pub async fn apply_events_and_persist<EvSnk, SsSnk, Ev, Agg, Evs, Mt, Repo, Ctx>(
        &self,
        agg: &mut HydratedAggregate<Agg>,
        events: Evs,
        meta: &Mt,
        repo: &Repo,
        ctx: Option<&Ctx>,
    ) -> Result<(), PersistError<EvSnk::Err, SsSnk::Err>>
    where
        Agg: Aggregate + EventSourced<Ev>,
        Ev: Event + 'static,
        Evs: AsRef<[Ev]>,
        Mt: ?Sized,
        EvSnk: EventSink<Agg, Ev, Mt> + ?Sized,
        SsSnk: SnapshotSink<Agg> + ?Sized,
        Repo: AsRef<EvSnk> + AsRef<SsSnk> + ?Sized,
        Ctx: BufferedContext + ?Sized,
    {
        let event_sink: &EvSnk = repo.as_ref();
        let events = event_sink
            .append_events(agg.id(), events.as_ref(), meta)
            .await
            .map_err(PersistError::Events)?;

        for ev in events {
            agg.apply(&ev);
            if let Some(c) = ctx {
                c.buffer_event(ev)
            }
        }

        self.persist_aggregate::<SsSnk, _, _>(agg, repo)
            .await
            .map_err(PersistError::Snapshot)
    }

    pub async fn exec_command_and_persist<EvSnk, SsSnk, Cmd, Mt, Repo, Ctx>(
        &self,
        cmd: Cmd,
        agg: Option<HydratedAggregate<Cmd::Aggregate>>,
        meta: &Mt,
        handler_ctx: &CommandHandlerContext<Cmd>,
        repo: &Repo,
        ctx: Option<&Ctx>,
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
        Repo: AsRef<EvSnk> + AsRef<SsSnk> + ?Sized,
        Ctx: BufferedContext + ?Sized,
    {
        let is_new = agg.is_none();
        let mut agg = agg.unwrap_or_default();
        let res = agg.state().handle(cmd, handler_ctx).await;
        match res {
            Ok(ev) => {
                let ev = ev.into_events();
                let events = ev.as_ref();
                if !events.is_empty() {
                    if is_new {
                        // TODO: reconsider
                        // For newly initiated `Aggregate` this is required,
                        // because it has no unique ID to persist it's `Event`s
                        // with. So, we should apply at least one `Event` to
                        // make it unique before storing its `Event`s.
                        agg.apply(NumberedEvent {
                            num: EventNumber::MIN_VALUE,
                            data: events.first().unwrap(),
                        });
                    }
                    self.apply_events_and_persist::<EvSnk, SsSnk, _, _, _, _, _, _>(
                        &mut agg, events, meta, repo, ctx,
                    )
                    .await?
                }
                Ok(agg)
            }
            Err(err) => Err(ExecAndPersistError::Exec(agg, err)),
        }
    }

    pub async fn load_aggregate_exec_command_and_persist<
        SsSrc,
        EvSrc,
        EvSnk,
        SsSnk,
        Cmd,
        Mt,
        Repo,
        Ctx,
    >(
        &self,
        cmd: Cmd,
        meta: &Mt,
        handler_ctx: &CommandHandlerContext<Cmd>,
        repo: &Repo,
        ctx: Option<&Ctx>,
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
        Repo: AsRef<SsSrc> + AsRef<EvSrc> + AsRef<EvSnk> + AsRef<SsSnk> + ?Sized,
        Ctx: BufferedContext + ?Sized,
    {
        let agg = if let Some(id) = cmd.aggregate_id() {
            let agg = self
                .load_aggregate_and_rehydrate::<SsSrc, EvSrc, _, _, _>(id, repo)
                .await?;
            if agg.is_none() {
                return Ok(None);
            }
            agg
        } else {
            Some(HydratedAggregate::default())
        };

        let agg = self
            .exec_command_and_persist::<EvSnk, SsSnk, _, _, _, _>(
                cmd,
                agg,
                meta,
                handler_ctx,
                repo,
                ctx,
            )
            .await?;
        Ok(Some(agg))
    }
}

#[derive(Clone, Copy, Debug, Display, Eq, Error, From, PartialEq)]
pub enum LoadRehydrateAndPersistError<SsSrcErr, EvSrcErr, SsSnkErr> {
    Load(LoadError<SsSrcErr, EvSrcErr>),
    #[display(fmt = "Persisting aggregate snapshot failed: {}", _0)]
    #[from(ignore)]
    Persist(SsSnkErr),
}

#[derive(Clone, Copy, Debug, Display, Eq, Error, PartialEq)]
pub enum PersistError<EvSnkErr, SsSnkErr> {
    #[display(fmt = "Persisting events failed: {}", _0)]
    Events(EvSnkErr),
    #[display(fmt = "Persisting aggregate snapshot failed: {}", _0)]
    Snapshot(SsSnkErr),
}

#[derive(Clone, Copy, Debug, Display, Eq, Error, From, PartialEq)]
pub enum ExecAndPersistError<Agg, CmdErr, EvSnkErr, SsSnkErr> {
    #[display(fmt = "Executing command failed: {}", _1)]
    #[from(ignore)]
    Exec(HydratedAggregate<Agg>, #[error(source)] CmdErr),
    Persist(PersistError<EvSnkErr, SsSnkErr>),
}

#[derive(Clone, Copy, Debug, Display, Eq, Error, From, PartialEq)]
pub enum LoadExecAndPersistError<Agg, CmdErr, SsSrcErr, EvSrcErr, EvSnkErr, SsSnkErr> {
    Load(LoadError<SsSrcErr, EvSrcErr>),
    #[display(fmt = "Executing command failed: {}", _1)]
    #[from(ignore)]
    Exec(HydratedAggregate<Agg>, #[error(source)] CmdErr),
    Persist(PersistError<EvSnkErr, SsSnkErr>),
}

impl<Agg, CmdErr, SsSrcErr, EvSrcErr, EvSnkErr, SsSnkErr>
    convert::From<ExecAndPersistError<Agg, CmdErr, EvSnkErr, SsSnkErr>>
    for LoadExecAndPersistError<Agg, CmdErr, SsSrcErr, EvSrcErr, EvSnkErr, SsSnkErr>
{
    #[inline]
    fn from(err: ExecAndPersistError<Agg, CmdErr, EvSnkErr, SsSnkErr>) -> Self {
        match err {
            ExecAndPersistError::Exec(agg, e) => Self::Exec(agg, e),
            ExecAndPersistError::Persist(e) => Self::Persist(e),
        }
    }
}
