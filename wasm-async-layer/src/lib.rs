use std::future::Future;

use csyncify::{asyncify, Token};
use wasm_runtime_layer::{AsContextMut, Func, FuncType, StoreContextMut, Value};
pub trait AsyncLayer {
    fn token(&self) -> Token;
    fn set_token(&mut self, t: Option<Token>) -> Option<Token>;
}
#[async_trait::async_trait]
pub trait AsyncFunc {
    fn new_async<C: AsContextMut, F: Future<Output = anyhow::Result<()>> + Send>(
        ctx: C,
        ty: FuncType,
        func: impl 'static
            + Send
            + Sync
            + Fn(StoreContextMut<'_, C::UserState, C::Engine>, &[Value], &mut [Value]) -> F,
    ) -> Self
    where
        C::UserState: AsyncLayer;
    async fn call_async<C: AsContextMut + Send>(
        &self,
        ctx: C,
        args: &[Value],
        results: &mut [Value],
    ) -> anyhow::Result<()>
    where
        C::UserState: AsyncLayer;
}
#[async_trait::async_trait]
impl AsyncFunc for Func {
    fn new_async<C: AsContextMut, F: Future<Output = anyhow::Result<()>> + Send>(
        ctx: C,
        ty: FuncType,
        func: impl 'static
            + Send
            + Sync
            + Fn(StoreContextMut<'_, C::UserState, C::Engine>, &[Value], &mut [Value]) -> F,
    ) -> Self
    where
        C::UserState: AsyncLayer,
    {
        return Self::new(ctx, ty, move |x, p, r| {
            x.data().token().block_on(func(x, p, r))
        });
    }

    async fn call_async<C: AsContextMut + Send>(
        &self,
        mut ctx: C,
        args: &[Value],
        results: &mut [Value],
    ) -> anyhow::Result<()>
    where
        C::UserState: AsyncLayer,
    {
        return asyncify(move |t| {
            let old = ctx.as_context_mut().data_mut().set_token(Some(t));
            let r = self.call(&mut ctx, args, results);
            ctx.as_context_mut().data_mut().set_token(old);
            return r;
        })
        .await;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
}
