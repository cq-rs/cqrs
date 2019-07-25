//! Conversions into fallible [`Future`].
//!
//! [`Future`]: futures::Future

#![allow(clippy::module_name_repetitions, clippy::type_complexity)]

use std::convert::Infallible;

use futures::{
    future::{self, BoxFuture},
    Future, FutureExt as _,
};

use super::{StaticFuture, StaticTryFuture};

/// Conversion to a [`Future`], which may fail.
pub trait IntoTryFuture<I, E> {
    /// Type that represents a fallible [`Future`].
    type Future: Future<Output = Result<I, E>>;

    /// Performs a conversion into the fallible [`Future`].
    fn into_try_future(self) -> Self::Future;
}

impl IntoTryFuture<(), Infallible> for () {
    type Future = future::Ready<Result<(), Infallible>>;

    #[inline]
    fn into_try_future(self) -> Self::Future {
        future::ok(())
    }
}

impl<I> IntoTryFuture<I, Infallible> for (I,) {
    type Future = future::Ready<Result<I, Infallible>>;

    #[inline]
    fn into_try_future(self) -> Self::Future {
        future::ok(self.0)
    }
}

impl<I> IntoTryFuture<Option<I>, Infallible> for Option<I> {
    type Future = future::Ready<Result<Self, Infallible>>;

    #[inline]
    fn into_try_future(self) -> Self::Future {
        future::ok(self)
    }
}

impl<I, E> IntoTryFuture<I, E> for Result<I, E> {
    type Future = future::Ready<Self>;

    #[inline]
    fn into_try_future(self) -> Self::Future {
        future::ready(self)
    }
}

impl<I, T> IntoTryFuture<I, Infallible> for StaticFuture<T>
where
    T: Future<Output = I>,
{
    type Future = future::Map<T, fn(I) -> Result<I, Infallible>>;

    #[inline]
    fn into_try_future(self) -> Self::Future {
        self.0.map(Ok)
    }
}

impl<I, E, T> IntoTryFuture<I, E> for StaticTryFuture<T>
where
    T: Future<Output = Result<I, E>>,
{
    type Future = T;

    #[inline]
    fn into_try_future(self) -> Self::Future {
        self.0
    }
}

impl<I> IntoTryFuture<I, Infallible> for BoxFuture<'_, I> {
    type Future = future::Map<Self, fn(I) -> Result<I, Infallible>>;

    #[inline]
    fn into_try_future(self) -> Self::Future {
        self.map(Ok)
    }
}

impl<I, E> IntoTryFuture<I, E> for BoxFuture<'_, Result<I, E>> {
    type Future = Self;

    #[inline]
    fn into_try_future(self) -> Self::Future {
        self
    }
}
