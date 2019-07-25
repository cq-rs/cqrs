//! Conversions into heavily used types.

mod try_future;
mod try_stream;

#[doc(inline)]
pub use self::{try_future::*, try_stream::*};

/// Trivial and transparent wrapper-type that provides [`IntoTryFuture`] and
/// [`IntoTryStream`] implementations for types which implement infallible
/// [`Future`] already, but don't implement [`IntoTryFuture`]/[`IntoTryStream`].
///
/// [`Future`]: futures::Future
#[allow(missing_debug_implementations)]
pub struct StaticFuture<T>(pub T);

impl<T> From<T> for StaticFuture<T> {
    #[inline]
    fn from(v: T) -> Self {
        Self(v)
    }
}

/// Trivial and transparent wrapper-type that provides [`IntoTryFuture`] and
/// [`IntoTryStream`] implementations for types which implement fallible
/// [`Future`] already, but don't implement [`IntoTryFuture`]/[`IntoTryStream`].
///
/// [`Future`]: futures::Future
#[allow(missing_debug_implementations)]
pub struct StaticTryFuture<T>(pub T);

impl<T> From<T> for StaticTryFuture<T> {
    #[inline]
    fn from(v: T) -> Self {
        Self(v)
    }
}
