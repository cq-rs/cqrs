use std::convert::Infallible;

use futures::{
    future::{self, BoxFuture},
    Future, FutureExt as _,
};

use super::EventNumber;

#[derive(Debug)]
pub struct StaticFuture<T>(T);

impl<T> StaticFuture<T> {
    #[inline(always)]
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for StaticFuture<T> {
    #[inline(always)]
    fn from(v: T) -> Self {
        Self(v)
    }
}

#[derive(Debug)]
pub struct StaticTryFuture<T>(T);

impl<T> StaticTryFuture<T> {
    #[inline(always)]
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for StaticTryFuture<T> {
    #[inline(always)]
    fn from(v: T) -> Self {
        Self(v)
    }
}

pub trait IntoTryFuture<I, E> {
    type Future: Future<Output = Result<I, E>>;

    fn into_try_future(self) -> Self::Future;
}

impl<I, E> IntoTryFuture<I, E> for Result<I, E> {
    type Future = future::Ready<Self>;

    #[inline(always)]
    fn into_try_future(self) -> Self::Future {
        future::ready(self)
    }
}

impl<I> IntoTryFuture<I, Infallible> for (I,) {
    type Future = future::Ready<Result<I, Infallible>>;

    #[inline(always)]
    fn into_try_future(self) -> Self::Future {
        future::ok(self.0)
    }
}

impl IntoTryFuture<(), Infallible> for () {
    type Future = future::Ready<Result<(), Infallible>>;

    #[inline(always)]
    fn into_try_future(self) -> Self::Future {
        future::ok(self)
    }
}

impl<I, T> IntoTryFuture<I, Infallible> for StaticFuture<T>
where
    T: Future<Output = I>,
{
    type Future = future::Map<T, fn(I) -> Result<I, Infallible>>;

    #[inline(always)]
    fn into_try_future(self) -> Self::Future {
        self.into_inner().map(Ok)
    }
}

impl<I, E, T> IntoTryFuture<I, E> for StaticTryFuture<T>
where
    T: Future<Output = Result<I, E>>,
{
    type Future = T;

    #[inline(always)]
    fn into_try_future(self) -> Self::Future {
        self.into_inner()
    }
}

impl<I, E> IntoTryFuture<I, E> for BoxFuture<'_, Result<I, E>> {
    type Future = Self;

    #[inline(always)]
    fn into_try_future(self) -> Self::Future {
        self
    }
}

impl<I> IntoTryFuture<I, Infallible> for BoxFuture<'_, I> {
    type Future = future::Map<Self, fn(I) -> Result<I, Infallible>>;

    #[inline(always)]
    fn into_try_future(self) -> Self::Future {
        self.map(Result::Ok)
    }
}
