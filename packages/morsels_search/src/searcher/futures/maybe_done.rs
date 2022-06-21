//! Definition of the MaybeDone combinator

use core::mem;
use core::pin::Pin;
use core::future::Future;
use std::task::{Context, Poll};

/// A future that may have completed.
///
/// This is created by the [`maybe_done()`] function.
pub enum MaybeDone<Fut: Future> {
    /// A not-yet-completed future
    Future(/* #[pin] */ Fut),
    /// The output of the completed future
    Done(Fut::Output),
    /// The empty variant after the result of a [`MaybeDone`] has been
    /// taken using the [`take_output`](MaybeDone::take_output) method.
    Gone,
}

impl<Fut: Future + Unpin> Unpin for MaybeDone<Fut> {}

impl<Fut: Future> MaybeDone<Fut> {
    /// Attempt to take the output of a `MaybeDone` without driving it
    /// towards completion.
    #[inline]
    pub fn take_output(self: Pin<&mut Self>) -> Option<Fut::Output> {
        match &*self {
            Self::Done(_) => {}
            Self::Future(_) | Self::Gone => return None,
        }
        unsafe {
            match mem::replace(self.get_unchecked_mut(), Self::Gone) {
                MaybeDone::Done(output) => Some(output),
                _ => unreachable!(),
            }
        }
    }
}

impl<Fut: Future> Future for MaybeDone<Fut> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            match self.as_mut().get_unchecked_mut() {
                MaybeDone::Future(f) => {
                    let res = match Pin::new_unchecked(f).poll(cx) {
                        Poll::Ready(t) => t,
                        Poll::Pending => return Poll::Pending,
                    };
                    self.set(Self::Done(res));
                }
                MaybeDone::Done(_) => {}
                MaybeDone::Gone => panic!("MaybeDone polled after value taken"),
            }
        }
        Poll::Ready(())
    }
}
