use super::{Aggregate, AggregatePrecondition, HydratedAggregate};
use super::query::AggregateQuery;
use error::ExecuteError;
use std::ops;
use std::error;
use std::marker::PhantomData;

pub trait Executor<Agg>
    where
        Agg: Aggregate,
{
    type AggregateId: ?Sized;
    type Error: error::Error;

    fn execute(&self, agg_id: &Self::AggregateId, command: Agg::Command, precondition: Option<AggregatePrecondition>) -> Result<AggregateWithNewEvents<Agg>, ExecuteError<Agg::CommandError, Self::Error>>;
}

pub struct AggregateWithNewEvents<Agg: Aggregate> {
    pub hydrated_aggregate: HydratedAggregate<Agg>,
    pub command_events: Agg::Events,
}

pub struct ViewExecutor<Agg, View>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
{
    view: View,
    _phantom: PhantomData<Agg>,
}

impl<Agg, View> ViewExecutor<Agg, View>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
{
    pub fn new(view: View) -> Self {
        ViewExecutor {
            view,
            _phantom: PhantomData,
        }
    }

    fn verify_precondition(state_opt: Option<HydratedAggregate<Agg>>, precondition: Option<AggregatePrecondition>) -> Result<HydratedAggregate<Agg>, ExecuteError<Agg::CommandError, View::Error>> {
        if let Some(precondition) = precondition {
            if let Some(state) = state_opt {
                match precondition {
                    AggregatePrecondition::Exists => Ok(state),
                    AggregatePrecondition::ExpectedVersion(v) if v == state.get_version() => Ok(state),
                    AggregatePrecondition::ExpectedVersion(_) | AggregatePrecondition::New =>
                        Err(ExecuteError::PreconditionFailed(precondition)),
                }
            } else if precondition == AggregatePrecondition::New {
                Ok(Default::default())
            } else {
                Err(ExecuteError::PreconditionFailed(precondition))
            }
        } else {
            Ok(state_opt.unwrap_or_default())
        }
    }
}

impl<Agg, View> Executor<Agg> for ViewExecutor<Agg, View>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
{
    type AggregateId = View::AggregateId;
    type Error = View::Error;

    fn execute(&self, agg_id: &Self::AggregateId, command: Agg::Command, precondition: Option<AggregatePrecondition>) -> Result<AggregateWithNewEvents<Agg>, ExecuteError<Agg::CommandError, Self::Error>> {
        let loaded_aggregate =
            self.view.rehydrate(agg_id)
                .map_err(ExecuteError::Load)?;

        let hydrated_aggregate = Self::verify_precondition(loaded_aggregate, precondition)?;

        let command_events =
            hydrated_aggregate.aggregate.execute(command)
                .map_err(ExecuteError::Command)?;

        Ok(AggregateWithNewEvents { hydrated_aggregate, command_events })
    }
}

impl<Agg, View> ops::Deref for ViewExecutor<Agg, View>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
{
    type Target = View;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl<Agg, View> From<View> for ViewExecutor<Agg, View>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
{
    fn from(view: View) -> Self {
        ViewExecutor::new(view)
    }
}