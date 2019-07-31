//! Conversions into fallible [`Stream`].
//!
//! [`Stream`]: futures::Stream

#![allow(clippy::module_name_repetitions, clippy::type_complexity)]

use std::{convert::Infallible, iter, vec, pin::Pin};

#[cfg(feature = "arrayvec")]
use arrayvec::{Array, ArrayVec};
use futures::{
    future::{self, BoxFuture, Either},
    stream::{self, BoxStream},
    Future, FutureExt as _, Stream, StreamExt as _, TryFutureExt as _,
};

use super::{StaticFuture, StaticTryFuture};

/// Conversion to a [`Stream`], which produces fallible output.
pub trait IntoTryStream<I, E> {
    /// Type that represents a fallible [`Stream`].
    type Stream: Stream<Item = Result<I, E>>;

    /// Performs a conversion into the fallible [`Stream`].
    fn into_try_stream(self) -> Self::Stream;
}

impl<I> IntoTryStream<I, Infallible> for () {
    type Stream = stream::Empty<Result<I, Infallible>>;

    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        stream::empty()
    }
}

impl<I> IntoTryStream<I, Infallible> for (I,) {
    type Stream = stream::Once<future::Ready<Result<I, Infallible>>>;

    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        stream::once(future::ok(self.0))
    }
}

impl<I> IntoTryStream<I, Infallible> for [I; 0] {
    type Stream = stream::Empty<Result<I, Infallible>>;

    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        stream::empty()
    }
}

// TODO: for single item mention std::iter::once, for empty mention std::iter::empty
impl<T, I> IntoTryStream<I, Infallible> for Option<T>
where
    T: IntoIterator<Item = I>,
{
    type Stream = Either<
        stream::Map<stream::Iter<T::IntoIter>, fn(I) -> Result<I, Infallible>>,
        stream::Empty<Result<I, Infallible>>,
    >;

    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        match self {
            Some(v) => Either::Left(stream::iter(v).map(Ok)),
            None => Either::Right(stream::empty()),
        }
    }
}

// TODO: for single item mention std::iter::once, for empty mention std::iter::empty
#[allow(clippy::use_self)]
impl<T, I, E> IntoTryStream<I, E> for Result<T, E>
where
    T: IntoIterator<Item = I>,
{
    type Stream = Either<
        stream::Map<stream::Iter<T::IntoIter>, fn(I) -> Result<I, E>>,
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

impl<I> IntoTryStream<I, Infallible> for Vec<I> {
    type Stream = stream::Map<stream::Iter<vec::IntoIter<I>>, fn(I) -> Result<I, Infallible>>;

    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        stream::iter(self).map(Ok)
    }
}

#[cfg(feature = "arrayvec")]
impl<A, I> IntoTryStream<I, Infallible> for ArrayVec<A>
where
    A: Array<Item = I>,
{
    type Stream = stream::Map<stream::Iter<arrayvec::IntoIter<A>>, fn(I) -> Result<I, Infallible>>;

    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        stream::iter(self).map(Ok)
    }
}

impl<T, O, I> IntoTryStream<I, Infallible> for StaticFuture<T>
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

impl<T, O, I, E> IntoTryStream<I, E> for StaticTryFuture<T>
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

impl<T, I> IntoTryStream<I, Infallible> for BoxFuture<'_, (T,)>
where
    T: IntoIterator<Item = I>,
{
    type Stream = stream::Map<
        future::FlattenStream<
            future::Map<
                future::Map<future::Map<Self, fn((T,)) -> T>, fn(T) -> T::IntoIter>,
                fn(T::IntoIter) -> stream::Iter<T::IntoIter>,
            >,
        >,
        fn(I) -> Result<I, Infallible>,
    >;

    #[allow(trivial_casts)]
    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        self.map(destruct_tuple as fn((T,)) -> T)
            .map(IntoIterator::into_iter as fn(T) -> T::IntoIter)
            .map(stream::iter as fn(T::IntoIter) -> stream::Iter<T::IntoIter>)
            .flatten_stream()
            .map(Ok)
    }
}

impl<T, I, E> IntoTryStream<I, E> for BoxFuture<'_, Result<T, E>>
where
    T: IntoIterator<Item = I>,
{
    type Stream = future::TryFlattenStream<
        future::MapOk<
            future::MapOk<Self, fn(T) -> iter::Map<T::IntoIter, fn(I) -> Result<I, E>>>,
            fn(
                iter::Map<T::IntoIter, fn(I) -> Result<I, E>>,
            ) -> stream::Iter<iter::Map<T::IntoIter, fn(I) -> Result<I, E>>>,
        >,
    >;

    #[allow(trivial_casts)]
    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        self.map_ok(to_result_iter as fn(T) -> iter::Map<T::IntoIter, fn(I) -> Result<I, E>>)
            .map_ok(
                stream::iter
                    as fn(
                        iter::Map<T::IntoIter, fn(I) -> Result<I, E>>,
                    )
                        -> stream::Iter<iter::Map<T::IntoIter, fn(I) -> Result<I, E>>>,
            )
            .try_flatten_stream()
    }
}

impl<I> IntoTryStream<I, Infallible> for BoxStream<'_, I> {
    type Stream = stream::Map<Self, fn(I) -> Result<I, Infallible>>;

    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        self.map(Ok)
    }
}

impl<I, E> IntoTryStream<I, E> for BoxStream<'_, Result<I, E>> {
    type Stream = Self;

    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        self
    }
}

type LocalBoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + 'a>>;

impl<I> IntoTryStream<I, Infallible> for LocalBoxStream<'_, I> {
    type Stream = stream::Map<Self, fn(I) -> Result<I, Infallible>>;

    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        self.map(Ok)
    }
}

impl<I, E> IntoTryStream<I, E> for LocalBoxStream<'_, Result<I, E>> {
    type Stream = Self;

    #[inline]
    fn into_try_stream(self) -> Self::Stream {
        self
    }
}

/// Converts given [`IntoIterator`] to an [`Iterator`] of successful [`Result`]s
/// as a static function.
///
/// Useful in places where such conversion should be passed as a function.
#[inline]
fn to_result_iter<T: IntoIterator<Item = I>, I, E>(
    items: T,
) -> iter::Map<T::IntoIter, fn(I) -> Result<I, E>> {
    items.into_iter().map(Ok)
}

/// Destructures a single-element tuple to its inner as a static function.
///
/// Useful in places where destructuring should be passed as a function.
#[inline]
fn destruct_tuple<T>(t: (T,)) -> T {
    t.0
}
