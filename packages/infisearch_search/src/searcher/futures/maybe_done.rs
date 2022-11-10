/* 
Copyright (c) 2016 Alex Crichton
Copyright (c) 2017 The Tokio Authors

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
*/

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
    pub fn take_output(self: Pin<&mut Self>) -> Option<Fut::Output> {
        match &*self {
            Self::Done(_) => {}
            Self::Future(_) | Self::Gone => return None,
        }
        unsafe {
            match mem::replace(self.get_unchecked_mut(), Self::Gone) {
                MaybeDone::Done(output) => Some(output),
                // unreachable
                _ => std::process::abort(),
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
                // MaybeDone polled after value taken
                MaybeDone::Gone => std::process::abort(),
            }
        }
        Poll::Ready(())
    }
}
