use std::pin::Pin;

use futures::{AsyncRead, AsyncSeek, AsyncWrite};

pub trait DynFile: AsyncRead + AsyncWrite + AsyncSeek + Send + Sync {}
impl<T: AsyncRead + AsyncWrite + AsyncSeek + Send + Sync> DynFile for T {}
pub enum Create {
    Dont,
    Directory,
    File,
}
pub enum FD<F, D> {
    File(F),
    Dir(D),
}
pub trait File:
    embedded_io_async::Read + embedded_io_async::Write + embedded_io_async::Seek
{
}
impl<T: embedded_io_async::Read + embedded_io_async::Write + embedded_io_async::Seek> File for T {}
#[async_trait::async_trait]
pub trait DirCommon {
    async fn erase(&mut self, a: String) -> std::io::Result<()>;
    async fn all(&self) -> std::io::Result<Vec<String>>;
}
#[async_trait::async_trait]
impl<T: DirCommon + Send + Sync + ?Sized> DirCommon for Box<T> {
    async fn erase(&mut self, a: String) -> std::io::Result<()> {
        return (&mut **self).erase(a).await;
    }
    async fn all(&self) -> std::io::Result<Vec<String>> {
        return (&**self).all().await;
    }
}
pub trait Dir: DirCommon {
    async fn open(
        &mut self,
        a: String,
        create: Create,
    ) -> std::io::Result<
        FD<
            impl File<Error = std::io::Error> + Send + Sync + 'static,
            impl Dir + Send + Sync + 'static,
        >,
    >;
}
#[async_trait::async_trait]
pub trait DynDir: Send + Sync + DirCommon {
    async fn dyn_open(
        &mut self,
        a: String,
        create: Create,
    ) -> std::io::Result<FD<Pin<Box<dyn DynFile>>, Box<dyn DynDir>>>;
}
#[async_trait::async_trait]
impl<T: Dir + Send + Sync> DynDir for T {
    async fn dyn_open(
        &mut self,
        a: String,
        create: Create,
    ) -> std::io::Result<FD<Pin<Box<dyn DynFile>>, Box<dyn DynDir>>> {
        let f = unsafe { embedded_io_convert::force_sync(self.open(a, create)) }.await?;
        return Ok(match f {
            FD::File(a) => FD::File(Box::pin(embedded_io_convert::read_write_seeeker(a))),
            FD::Dir(b) => FD::Dir(Box::new(b)),
        });
    }
}

impl<T: DynDir + ?Sized> Dir for Box<T> {
    async fn open(
        &mut self,
        a: String,
        create: Create,
    ) -> std::io::Result<
        FD<
            impl File<Error = std::io::Error> + Send + Sync + 'static,
            impl Dir + Send + Sync + 'static,
        >,
    > {
        let o2 = self.dyn_open(a, create).await?;
        return Ok(match o2 {
            FD::File(f) => FD::File(embedded_io_convert::from::FromFutures::new(f)),
            FD::Dir(d) => FD::Dir(d),
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
}
