use std::sync::Mutex;

use minicoroutine::{Coroutine, CoroutineRef, GLOBAL};
use pasts::{prelude::*, Loop};

pub struct Exit;

type Cor<R, W> = Coroutine<W, Pin<Box<dyn Future<Output = W> + Send>>, R, ()>;
type CorRef<R, W> = CoroutineRef<W, Pin<Box<dyn Future<Output = W> + Send>>, R, (), GLOBAL>;
#[derive(Clone)]
pub struct Token {
    internal: CorRef<T, T>,
}
impl Token {
    pub fn block_on<T>(&self, a: impl Future<Output = T> + Send) -> T {
        return block_on_in(self.internal.clone(), a);
    }
    pub unsafe fn current() -> Self {
        return Token {
            internal: Cor::running().unwrap(),
        };
    }
}
unsafe impl Send for Token{}
struct Cad<R, W> {
    pub r: Cor<R, W>,
    cur: Option<W>,
}
impl<R, W> Cad<R, W> {
    pub fn new(r: Cor<R, W>, w: W) -> Self {
        Self { r: r, cur: Some(w) }
    }
}
impl<R> Cad<R, T> {
    pub fn from_fn(f: impl Fn(CorRef<R, T>) -> R + Send + 'static) -> Self {
        return Self::new(Cor::new(f, ()).unwrap(), T(0 as *mut ()));
    }
}
unsafe impl<R,W> Send for Cad<R,W>{}
fn block_on_any_in<R, W>(mut r: CorRef<R, W>, a: impl Future<Output = W> + Send) -> W {
    let b: Pin<Box<dyn Future<Output = W> + Send>> = Box::pin(a);
    return r.yield_(b);
}
// pub fn block_on_any<R, W>(a: impl Future<Output = W> + Send) -> W {
//     return block_on_any_in::<R, W>(Cor::running().unwrap(), a);
// }
fn block_on_in<R, T>(r: CorRef<R, crate::T>, a: impl Future<Output = T> + Send) -> T {
    let b = block_on_any_in(r, async move {
        let b = a.await;
        return crate::T(Box::into_raw(Box::new(b)) as *mut ());
    });
    return *unsafe { Box::from_raw(b.0 as *mut T) };
}
// pub fn block_on<R,W>(a: impl Future<Output = W> + Send) -> W{
//     return block_on_in::<R,W>(Cor::running().unwrap(), a);
// }
async fn x<R, W>(x: &mut Cad<R, W>) -> Option<R> {
    // let mut a = Default::default();
    loop {
        let m = x.r.resume(x.cur.take()?)?;
        let m = match m {
            minicoroutine::CoroutineResult::Yield(y) => y,
            minicoroutine::CoroutineResult::Return(r) => return Some(r),
            minicoroutine::CoroutineResult::Error(_) => return None,
        };
        x.cur = Some(m.await);
    }
}
async fn x_own<R, W>(mut xv: Cad<R, W>) -> Option<R> {
    return x(&mut xv).await;
}
async fn asyncify_<R: 'static>(f: impl FnOnce(Token) -> R + Send + 'static) -> R {
    let f = Mutex::new(Some(f));
    let a = x_own(Cad::from_fn(move |a| {
        let a = f.lock().unwrap().take().unwrap()(Token { internal: a });
        return T(Box::into_raw(Box::new(a)) as *mut ());
    }))
    .await
    .unwrap();
    return *unsafe { Box::from_raw(a.0 as *mut R) };
}
struct T(*mut ());
unsafe impl Send for T{}
impl T{
    unsafe fn into_other<U>(self) -> U{
        return *Box::from_raw(self.0 as *mut U);
    }
}
pub async fn asyncify<R: 'static, F: FnOnce(Token) -> R>(f: F) -> R {
    let p = Box::into_raw(Box::new(f));
    let p = T(p as *mut ());
    return asyncify_(move |t| (unsafe{p.into_other::<F>()})(t)).await;
}
#[cfg(test)]
mod tests {
    use super::*;
}
