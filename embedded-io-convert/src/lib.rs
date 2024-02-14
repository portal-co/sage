use std::{pin::Pin, sync::Mutex};

use embedded_io_async::ErrorType;
use futures::{AsyncRead, AsyncSeek, AsyncWrite, Future};

pub mod mutex;
pub mod read;
pub mod write;
pub mod seek;
pub use embedded_io_adapters::futures_03 as from;
struct MutexFuture<T>(Mutex<Pin<Box<T>>>);
impl<T: Future> Future for MutexFuture<T> {
    type Output = T::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        return self.0.lock().unwrap().as_mut().poll(cx);
    }
}
impl<T> MutexFuture<T> {
    pub fn new(t: T) -> MutexFuture<T> {
        return MutexFuture(Mutex::new(Box::pin(t)));
    }
}
//SAFETY: TODO
unsafe impl<T> Send for MutexFuture<T> {}
unsafe impl<T> Sync for MutexFuture<T> {}
pub unsafe fn force_sync<T: Future>(t: T) -> impl Future<Output = T::Output> + Send + Sync{
    return MutexFuture::new(t);
}

pub fn read_writer<E: embedded_io_async::Read + embedded_io_async::Write + Send + Sync + 'static>(
    a: E,
) -> impl AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static
where
    <E as ErrorType>::Error: Into<std::io::Error> + Send + Sync,
{
    let a = mutex::Mutexed::new(a);
    let b = a.clone();
    let a = read::SimpleAsyncReader::new(a);
    let b = write::SimpleAsyncWriter::new(b);
    return merge_io::MergeIO::new(a, b);
}
pub fn read_write_seeeker<E: embedded_io_async::Read + embedded_io_async::Write + embedded_io_async::Seek + Send + Sync + 'static>(
    a: E,
) -> impl AsyncRead + AsyncWrite + AsyncSeek + Unpin + Send + Sync + 'static
where
    <E as ErrorType>::Error: Into<std::io::Error> + Send + Sync,
{
    let a = mutex::Mutexed::new(a);
    let b = a.clone();
    let a = read_writer(a);
    let b = seek::SimpleAsyncSeeker::new(b);
    return MergeSeek{readwrite: a, seek: b};
}
pub struct MergeSeek<RW,S>{
    pub readwrite: RW,
    pub seek: S,
}
impl<RW: AsyncRead + Unpin,S: Unpin> AsyncRead for MergeSeek<RW,S>{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        return AsyncRead::poll_read(Pin::new(&mut self.get_mut().readwrite), cx, buf);
    }
    fn poll_read_vectored(
                self: Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
                bufs: &mut [std::io::IoSliceMut<'_>],
            ) -> std::task::Poll<std::io::Result<usize>> {
                return AsyncRead::poll_read_vectored(Pin::new(&mut self.get_mut().readwrite), cx, bufs);
    }
}
impl<RW: AsyncWrite+ Unpin,S: Unpin> AsyncWrite for MergeSeek<RW,S>{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        return AsyncWrite::poll_write(Pin::new(&mut self.get_mut().readwrite), cx, buf);
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        return AsyncWrite::poll_flush(Pin::new(&mut self.get_mut().readwrite), cx);
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        return AsyncWrite::poll_close(Pin::new(&mut self.get_mut().readwrite), cx);
    }
    fn poll_write_vectored(
                self: Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
                bufs: &[std::io::IoSlice<'_>],
            ) -> std::task::Poll<std::io::Result<usize>> {
                return AsyncWrite::poll_write_vectored(Pin::new(&mut self.get_mut().readwrite), cx, bufs);
    }
}
impl<RW: Unpin,S: AsyncSeek + Unpin> AsyncSeek for MergeSeek<RW,S>{
    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        pos: std::io::SeekFrom,
    ) -> std::task::Poll<std::io::Result<u64>> {
        return AsyncSeek::poll_seek(Pin::new(&mut self.get_mut().seek), cx, pos);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
}
