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

//! Definition of the `JoinAll` combinator, waiting for all of a list of futures
//! to finish.

use core::future::Future;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll};

use super::maybe_done::MaybeDone;

fn iter_pin_mut<T>(slice: Pin<&mut [T]>) -> impl Iterator<Item = Pin<&mut T>> {
    // Safety: `std` _could_ make this unsound if it were to decide Pin's
    // invariants aren't required to transmit through slices. Otherwise this has
    // the same safety as a normal field pin projection.
    unsafe { slice.get_unchecked_mut() }.iter_mut().map(|t| unsafe { Pin::new_unchecked(t) })
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
/// Future for the [`join_all`] function.
pub struct JoinAll<F>
where
    F: Future,
{
    elems: Pin<Box<[MaybeDone<F>]>>,
}

pub fn join_all<I>(iter: I) -> JoinAll<I::Item>
where
    I: IntoIterator,
    I::Item: Future,
{
    let elems = iter.into_iter().map(MaybeDone::Future).collect::<Box<[_]>>().into();
    JoinAll { elems }
}

impl<F> Future for JoinAll<F>
where
    F: Future,
{
    type Output = Vec<F::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;

        for elem in iter_pin_mut(self.elems.as_mut()) {
            if elem.poll(cx).is_pending() {
                all_done = false;
            }
        }

        if all_done {
            let mut elems = mem::replace(&mut self.elems, Box::pin([]));
            let result = iter_pin_mut(elems.as_mut())
                .map(|e| unsafe { e.take_output().unwrap_unchecked() }).collect();
            Poll::Ready(result)
        } else {
            Poll::Pending
        }
    }
}
