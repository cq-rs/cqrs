use std::{convert::Infallible, iter, vec};

use futures::{
    future::{self, BoxFuture, Either},
    stream::{self, BoxStream},
    Future, FutureExt as _, Stream, StreamExt as _, TryFutureExt as _,
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

#[derive(Debug)]
pub struct StaticIterFuture<T>(T);

impl<T> StaticIterFuture<T> {
    #[inline(always)]
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for StaticIterFuture<T> {
    #[inline(always)]
    fn from(v: T) -> Self {
        Self(v)
    }
}

#[derive(Debug)]
pub struct StaticIterTryFuture<T>(T);

impl<T> StaticIterTryFuture<T> {
    #[inline(always)]
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for StaticIterTryFuture<T> {
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

impl<I> IntoTryFuture<Option<I>, Infallible> for Option<I> {
    type Future = future::Ready<Result<Self, Infallible>>;

    #[inline(always)]
    fn into_try_future(self) -> Self::Future {
        future::ok(self)
    }
}

impl IntoTryFuture<(), Infallible> for () {
    type Future = future::Ready<Result<(), Infallible>>;

    #[inline(always)]
    fn into_try_future(self) -> Self::Future {
        future::ok(self)
    }
}

impl<I> IntoTryFuture<I, Infallible> for (I,) {
    type Future = future::Ready<Result<I, Infallible>>;

    #[inline(always)]
    fn into_try_future(self) -> Self::Future {
        future::ok(self.0)
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
        self.map(Ok)
    }
}

pub trait IntoTryStream<I, E> {
    type Stream: Stream<Item = Result<I, E>>;

    fn into_try_stream(self) -> Self::Stream;
}

impl<I, E> IntoTryStream<I, E> for Result<I, E> {
    type Stream = stream::Once<future::Ready<Self>>;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        stream::once(future::ready(self))
    }
}

impl<I> IntoTryStream<Option<I>, Infallible> for Option<I> {
    type Stream = stream::Once<future::Ready<Result<Self, Infallible>>>;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        stream::once(future::ok(self))
    }
}

impl<I> IntoTryStream<I, Infallible> for () {
    type Stream = stream::Empty<Result<I, Infallible>>;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        stream::empty()
    }
}

impl<I> IntoTryStream<I, Infallible> for (I,) {
    type Stream = stream::Once<future::Ready<Result<I, Infallible>>>;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        stream::once(future::ok(self.0))
    }
}

impl<I> IntoTryStream<I, Infallible> for [I; 0] {
    type Stream = stream::Empty<Result<I, Infallible>>;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        stream::empty()
    }
}

impl<I> IntoTryStream<I, Infallible> for Vec<I> {
    type Stream = stream::Map<stream::Iter<vec::IntoIter<I>>, fn(I) -> Result<I, Infallible>>;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        stream::iter(self).map(Ok)
    }
}

impl<I, E> IntoTryStream<I, E> for Result<Vec<I>, E> {
    type Stream = Either<
        stream::Map<stream::Iter<vec::IntoIter<I>>, fn(I) -> Result<I, E>>,
        stream::Once<future::Ready<Result<I, E>>>,
    >;

    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        match self {
            Ok(v) => Either::Left(stream::iter(v).map(Ok)),
            Err(e) => Either::Right(stream::once(future::err(e))),
        }
    }
}

impl<T, I, E> IntoTryStream<I, E> for StaticTryFuture<T>
where
    T: Future<Output = Result<I, E>>,
{
    type Stream = stream::Once<T>;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        stream::once(self.0)
    }
}

impl<T, O, I, E> IntoTryStream<I, E> for StaticIterTryFuture<T>
where
    T: Future<Output = Result<O, E>>,
    O: IntoIterator<Item = I>,
{
    type Stream = future::TryFlattenStream<
        future::MapOk<
            future::MapOk<T, fn(O) -> iter::Map<O::IntoIter, fn(I) -> Result<I, E>>>,
            fn(
                iter::Map<O::IntoIter, fn(I) -> Result<I, E>>,
            ) -> stream::Iter<iter::Map<O::IntoIter, fn(I) -> Result<I, E>>>,
        >,
    >;

    #[allow(trivial_casts)]
    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        self.0
            .map_ok(to_result_iter as fn(O) -> iter::Map<O::IntoIter, fn(I) -> Result<I, E>>)
            .map_ok(
                stream::iter
                    as fn(
                        iter::Map<O::IntoIter, fn(I) -> Result<I, E>>,
                    )
                        -> stream::Iter<iter::Map<O::IntoIter, fn(I) -> Result<I, E>>>,
            )
            .try_flatten_stream()
    }
}

impl<T, I> IntoTryStream<I, Infallible> for StaticFuture<T>
where
    T: Future<Output = I>,
{
    type Stream = stream::Map<stream::Once<T>, fn(I) -> Result<I, Infallible>>;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        stream::once(self.0).map(Ok)
    }
}

impl<T, O, I> IntoTryStream<I, Infallible> for StaticIterFuture<T>
where
    T: Future<Output = O>,
    O: IntoIterator<Item = I>,
{
    type Stream = stream::Map<
        future::FlattenStream<
            future::Map<
                future::Map<T, fn(O) -> O::IntoIter>,
                fn(O::IntoIter) -> stream::Iter<O::IntoIter>,
            >,
        >,
        fn(I) -> Result<I, Infallible>,
    >;

    #[allow(trivial_casts)]
    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        self.0
            .map(IntoIterator::into_iter as fn(O) -> O::IntoIter)
            .map(stream::iter as fn(O::IntoIter) -> stream::Iter<O::IntoIter>)
            .flatten_stream()
            .map(Ok)
    }
}

impl<I, E> IntoTryStream<I, E> for BoxFuture<'_, Result<I, E>> {
    type Stream = stream::Once<Self>;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        stream::once(self)
    }
}

impl<I, E> IntoTryStream<I, E> for BoxFuture<'_, Result<Vec<I>, E>> {
    type Stream = future::TryFlattenStream<
        future::MapOk<
            future::MapOk<Self, fn(Vec<I>) -> iter::Map<vec::IntoIter<I>, fn(I) -> Result<I, E>>>,
            fn(
                iter::Map<vec::IntoIter<I>, fn(I) -> Result<I, E>>,
            ) -> stream::Iter<iter::Map<vec::IntoIter<I>, fn(I) -> Result<I, E>>>,
        >,
    >;

    #[allow(trivial_casts)]
    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        self.map_ok(
            to_result_iter as fn(Vec<I>) -> iter::Map<vec::IntoIter<I>, fn(I) -> Result<I, E>>,
        )
        .map_ok(
            stream::iter
                as fn(
                    iter::Map<vec::IntoIter<I>, fn(I) -> Result<I, E>>,
                )
                    -> stream::Iter<iter::Map<vec::IntoIter<I>, fn(I) -> Result<I, E>>>,
        )
        .try_flatten_stream()
    }
}

impl<I> IntoTryStream<I, Infallible> for BoxFuture<'_, I> {
    type Stream = stream::Once<future::Map<Self, fn(I) -> Result<I, Infallible>>>;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        stream::once(self.map(Ok))
    }
}

impl<I> IntoTryStream<I, Infallible> for BoxFuture<'_, Vec<I>> {
    type Stream = stream::Map<
        future::FlattenStream<future::Map<Self, fn(Vec<I>) -> stream::Iter<vec::IntoIter<I>>>>,
        fn(I) -> Result<I, Infallible>,
    >;

    #[allow(trivial_casts)]
    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        self.map(stream::iter as fn(Vec<I>) -> stream::Iter<vec::IntoIter<I>>)
            .flatten_stream()
            .map(Ok)
    }
}

impl<I, E> IntoTryStream<I, E> for BoxStream<'_, Result<I, E>> {
    type Stream = Self;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        self
    }
}

impl<I> IntoTryStream<I, Infallible> for BoxStream<'_, I> {
    type Stream = stream::Map<Self, fn(I) -> Result<I, Infallible>>;

    #[inline(always)]
    fn into_try_stream(self) -> Self::Stream {
        self.map(Ok)
    }
}

fn to_result_iter<T: IntoIterator<Item = I>, I, E>(
    items: T,
) -> iter::Map<T::IntoIter, fn(I) -> Result<I, E>> {
    items.into_iter().map(Ok)
}
