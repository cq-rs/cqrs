//! Command related definitions.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;

use super::{Aggregate, Event, IntoEvents, Version};

/// [CQRS] command that describes an intent to change the [`Aggregate`]'s state.
///
/// A state change within an application starts with a [`Command`].
/// A [`Command`] is a combination of expressed intent (which describes what
/// you want to do) as well as the information required to undertake action
/// based on that intent. The [`CommandHandler`] is used to process the incoming
/// [`Command`] for some [`Aggregate`], to validate it and to define the outcome
/// (an [`Event`], usually).
///
/// [CQRS]: https://martinfowler.com/bliki/CQRS.html
pub trait Command {
    /// Type of [`Aggregate`] that this [`Command`] should be handled for.
    type Aggregate: Aggregate;

    /// Returns ID of the [`Aggregate`] that this [`Command`] is addressed to.
    ///
    /// `None` means that a new [`Aggregate`] should be initialized for
    /// this [`Command`].
    #[inline(always)]
    fn aggregate_id(&self) -> Option<&<Self::Aggregate as Aggregate>::Id> {
        None
    }

    /// Returns expected [`Version`] of the [`Aggregate`] that this [`Command`]
    /// is addressed to.
    ///
    /// `None` means that [`Command`] may be handled for any [`Version`] of the
    /// [`Aggregate`].
    ///
    /// Has no effect if [`Command::aggregate_id`] returns `None`.
    #[inline(always)]
    fn expected_version(&self) -> Option<Version> {
        None
    }
}

/// Handler of a specific [`Command`] that processes it for its [`Aggregate`].
#[async_trait]
pub trait CommandHandler<C: Command> {
    /// Type of context required by this [`CommandHandler`] for performing
    /// an operation.
    ///
    /// This should be used to provide any additional services/resources
    /// required to handle the [`Command`].
    type Context: ?Sized;
    /// Type of [`Event`] produced as a result of the [`Command`] handling.
    type Event: Event;
    /// Type of the error if [`Command`] handling fails. If it never fails,
    /// consider to specify [`Infallible`].
    ///
    /// [`Infallible`]: std::convert::Infallible.
    type Err;
    /// Type of the result of successful [`Command`] handling. It should be
    /// convertible into [`Event`]s collection (see [`IntoEvents`] trait for
    /// details).
    type Ok: IntoEvents<Self::Event>;

    /// Handles and processes given [`Command`] for its [`Aggregate`].
    async fn handle_command(&self, cmd: C, ctx: &Self::Context) -> Result<Self::Ok, Self::Err>;
}
