use std::{
    collections::{BTreeMap, VecDeque},
    pin::Pin,
};

use anyhow::Context;
use embedded_io_async::ErrorType;
use futures::{AsyncReadExt, AsyncWriteExt};

use crate::{
    packet::{Handler, Pusher, ARWS, IO, Server},
    Id,
};

pub struct SocketHandler {
    pub xs: BTreeMap<[u8; 32], Pin<Box<dyn ARWS>>>,
}
pub struct MergeHandler {
    pub sock: SocketHandler,
}
pub async fn merge(mut p: impl Pusher + Send + Sync + 'static, x: Id,pre: &[u8]) -> anyhow::Result<impl Server + Send + Sync + 'static>{
    let mut v1 = pre.to_vec();
    v1.push(b'R');
    let a = p.push_encrypted(x,v1).await?;
    let mut v2 = pre.to_vec();
    v2.push(b'S');
    return Ok(IO(embedded_io_convert::read_writer(Socket{
        pusher: p,
        prefix: v2,
        target: x,
        stream: (&a as &[u8]).try_into()?
    })));
}
#[async_trait::async_trait]
impl Handler for MergeHandler {
    async fn handle(
        &mut self,
        mut p: Vec<u8>,
        a: &mut VecDeque<Box<dyn Pusher + Send>>,
    ) -> anyhow::Result<Vec<u8>> {
        if p[0] == b'S' {
            return self.sock.handle(p[1..].to_vec(), a).await;
        }
        if p[0] == b'R' {
            let (ra, wa) = sluice::pipe::pipe();
            let (rb, wb) = sluice::pipe::pipe();
            let aa = self.sock.alloc(Box::pin(merge_io::MergeIO::new(ra, wb)));
            a.push_back(Box::new(crate::packet::IO(merge_io::MergeIO::new(rb, wa))));
            return Ok(aa.to_vec());
        }
        return Ok(vec![]);
    }
}
impl SocketHandler {
    pub fn alloc(&mut self, a: Pin<Box<dyn ARWS>>) -> [u8; 32] {
        let r = rand::random();
        self.xs.insert(r, a);
        return r;
    }
}
#[async_trait::async_trait]
impl Handler for SocketHandler {
    async fn handle(
        &mut self,
        mut p: Vec<u8>,
        a: &mut VecDeque<Box<dyn Pusher + Send>>,
    ) -> anyhow::Result<Vec<u8>> {
        let mut x = self.xs.get_mut(&p[..32]).context("socket not found")?;
        p = p[32..].to_vec();
        let read = p[0] == 0;
        if read {
            let size = u32::from_be_bytes(p[1..5].try_into()?);
            let mut v = vec![0; size as usize];
            x.read_exact(&mut v).await?;
            return Ok(v);
        }
        x.write_all(&p[1..]).await?;
        x.flush().await?;
        return Ok(vec![]);
    }
}
#[derive(Clone)]
pub struct Socket<P> {
    pub pusher: P,
    pub prefix: Vec<u8>,
    pub target: Id,
    pub stream: [u8; 32],
}
impl<T: Pusher> ErrorType for Socket<T> {
    type Error = std::io::Error;
}
impl<T: Pusher + Send> embedded_io_async::Read for Socket<T> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut r = self.prefix.clone();
        r.extend(self.stream);
        r.push(0);
        r.extend(u32::to_be_bytes(buf.len().try_into().unwrap()));
        buf.copy_from_slice(
            &self
                .pusher
                .push_encrypted(self.target, r)
                .await
                .map_err(|e| std::io::Error::other(e))?,
        );
        return Ok(buf.len());
    }
}
impl<T: Pusher + Send> embedded_io_async::Write for Socket<T> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut r = self.prefix.clone();
        r.extend(self.stream);
        r.push(1);
        r.extend(buf);
        self.pusher
            .push_encrypted(self.target, r)
            .await
            .map_err(|e| std::io::Error::other(e))?;
        return Ok(buf.len());
    }
}
