mod basic;
mod context;
mod r#static;

use cqrs_core::{Command, CommandHandler, EventSink, EventSource, SnapshotSink, SnapshotSource};

#[doc(inline)]
pub use self::{
    basic::{
        Basic, ExecAndPersistError, LoadError, LoadExecAndPersistError,
        LoadRehydrateAndPersistError, PersistError,
    },
    context::{BorrowableAsContext, BufferedContext, Context, ContextWithMeta},
    r#static::Static,
};

type CommandHandlerErr<Cmd> = <<Cmd as Command>::Aggregate as CommandHandler<Cmd>>::Err;
type CommandHandlerEvent<Cmd> = <<Cmd as Command>::Aggregate as CommandHandler<Cmd>>::Event;
type CommandHandlerContext<Cmd> = <<Cmd as Command>::Aggregate as CommandHandler<Cmd>>::Context;

type EventSinkErr<Impl, Cmd, Mt> =
    <Impl as EventSink<<Cmd as Command>::Aggregate, CommandHandlerEvent<Cmd>, Mt>>::Err;
type EventSourceErr<Impl, Cmd> =
    <Impl as EventSource<<Cmd as Command>::Aggregate, CommandHandlerEvent<Cmd>>>::Err;
type SnapshotSinkErr<Impl, Cmd> = <Impl as SnapshotSink<<Cmd as Command>::Aggregate>>::Err;
type SnapshotSourceErr<Impl, Cmd> = <Impl as SnapshotSource<<Cmd as Command>::Aggregate>>::Err;
