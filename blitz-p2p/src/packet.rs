use std::{
    collections::{BTreeSet, VecDeque},
    convert::Infallible,
    pin::Pin,
    sync::Arc,
};

// use crypto::curve25519::curve25519_base;
use dyn_clone::DynClone;
use embedded_io_async::ReadExactError;
use futures::{lock::Mutex, AsyncRead, AsyncWrite};
use rand::Rng;
use rand::RngCore;
use serde::{Serialize, Deserialize};
use simple_encryption::{decrypt, x25519_base};
use whisk::Channel;

use crate::Id;
pub async fn decode_vec<R: embedded_io_async::Read>(
    r: &mut R,
) -> Result<Vec<u8>, ReadExactError<R::Error>> {
    let mut data_len = [0u8; 4];

    r.read_exact(&mut data_len).await?;
    let data_len = u32::from_be_bytes(data_len);
    let mut data = vec![0u8; data_len as usize];
    r.read_exact(&mut data).await?;
    return Ok(data);
}
pub async fn encode_vec<W: embedded_io_async::Write>(a: &[u8], w: &mut W) -> Result<(), W::Error> {
    w.write_all(&u32::to_be_bytes(a.len() as u32)).await?;
    w.write_all(a).await?;
    return Ok(());
}
#[derive(Clone,Serialize,Deserialize)]
pub struct Packet {
    taint: BTreeSet<Id>,
    target: Id,
    data: Vec<u8>,
}
impl Packet {
    pub fn encrypted(target: Id, data: Vec<u8>) -> anyhow::Result<Packet> {
        let d = simple_encryption::encrypt(&target, &data)?;
        return Ok(Packet {
            taint: BTreeSet::new(),
            target: target,
            data: d,
        });
    }
    pub async fn decode<R: embedded_io_async::Read>(
        r: &mut R,
    ) -> Result<Packet, ReadExactError<R::Error>> {
        let mut taint = BTreeSet::new();
        let mut taint_len = [0u8; 4];
        r.read_exact(&mut taint_len).await?;
        let taint_len = u32::from_be_bytes(taint_len);
        for _ in 0..taint_len {
            let mut i = Id::default();
            r.read_exact(&mut i).await?;
            taint.insert(i);
        }
        let mut target = Id::default();
        r.read_exact(&mut target).await?;
        let data = decode_vec(r).await?;
        return Ok(Packet {
            taint: taint,
            target: target,
            data: data,
        });
    }
    pub async fn encode<W: embedded_io_async::Write>(&self, w: &mut W) -> Result<(), W::Error> {
        w.write_all(&u32::to_be_bytes(self.taint.len() as u32))
            .await?;
        for t in &self.taint {
            w.write_all(t).await?;
        }
        w.write_all(&self.target).await?;
        encode_vec(&self.data, w).await?;
        return Ok(());
    }
}
#[async_trait::async_trait]
pub trait Handler {
    async fn handle(
        &mut self,
        p: Vec<u8>,
        a: &mut VecDeque<Box<dyn Pusher + Send>>,
    ) -> anyhow::Result<Vec<u8>>;
}
pub struct Dispatch(pub [Box<dyn Handler + Send>; 256]);
#[async_trait::async_trait]
impl Handler for Dispatch {
    async fn handle(
        &mut self,
        p: Vec<u8>,
        a: &mut VecDeque<Box<dyn Pusher + Send>>,
    ) -> anyhow::Result<Vec<u8>> {
        return self.0[p[0] as usize].handle(p[1..].to_vec(), a).await;
    }
}
#[async_trait::async_trait]
pub trait Pusher {
    async fn push(&mut self, a: Packet) -> anyhow::Result<Vec<u8>>;
    async fn push_encrypted(&mut self, target: Id, data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let mut rng = ::rand::rngs::OsRng::default();
        let mut k = [0u8; 32];
        rng.fill_bytes(&mut k);
        let r = x25519_base(k)
            .iter()
            .map(|a| *a)
            .chain(data.into_iter())
            .collect();
        let r = Packet::encrypted(target, r)?;
        let mut d = self.push(r).await?;
        d.pop();
        return decrypt(&k, &d).map_err(|e| e.into());
    }
}
pub trait PusherClone: Pusher + DynClone{}
impl<T: Pusher + DynClone> PusherClone for T{}
dyn_clone::clone_trait_object!(PusherClone);
#[async_trait::async_trait]
impl<T: Pusher + Send> Pusher for Arc<Mutex<T>> {
    async fn push(&mut self, a: Packet) -> anyhow::Result<Vec<u8>> {
        return self.lock().await.push(a).await;
    }
}

#[async_trait::async_trait]
pub trait Server {
    async fn serve(&mut self, x: &mut (dyn Pusher + Send)) -> anyhow::Result<Infallible>;
}
#[async_trait::async_trait]
impl<T: Server + ?Sized + Send> Server for Box<T> {
    async fn serve(&mut self, x: &mut (dyn Pusher + Send)) -> anyhow::Result<Infallible> {
        return (&mut **self).serve(x).await;
    }
}
pub struct PushChan {
    pub packet: Packet,
    pub ret: Channel<anyhow::Result<Vec<u8>>>,
}
#[async_trait::async_trait]
impl Pusher for Channel<PushChan> {
    async fn push(&mut self, a: Packet) -> anyhow::Result<Vec<u8>> {
        let n = Channel::new();
        self.send(PushChan {
            packet: a,
            ret: n.clone(),
        })
        .await;
        return n.recv().await;
    }
}
#[async_trait::async_trait]
impl Server for Channel<PushChan> {
    async fn serve(&mut self, x: &mut (dyn Pusher + Send)) -> anyhow::Result<Infallible> {
        loop {
            let a = self.recv().await;
            a.ret.send(x.push(a.packet).await).await;
        }
    }
}
pub struct IO<T>(pub T);
#[async_trait::async_trait]
impl<T: ARWS + Unpin + Send> Pusher for IO<T> {
    async fn push(&mut self, a: Packet) -> anyhow::Result<Vec<u8>> {
        let mut s = embedded_io_convert::from::FromFutures::new(&mut self.0);
        a.encode(&mut s).await?;

        return Ok(decode_vec(&mut s).await?);
    }
}
#[async_trait::async_trait]
impl<T: ARWS + Unpin + Send> Server for IO<T> {
    async fn serve(&mut self, x: &mut (dyn Pusher + Send)) -> anyhow::Result<Infallible> {
        let mut s = embedded_io_convert::from::FromFutures::new(&mut self.0);
        loop {
            let p = Packet::decode(&mut s).await?;
            let r = x.push(p).await?;
            encode_vec(&r, &mut s).await?;
        }
    }
}
pub trait ARWS: AsyncRead + AsyncWrite + Send + Sync + 'static {}
impl<T: AsyncRead + AsyncWrite + Send + Sync + 'static> ARWS for T {}
// pub struct Target(pub embedded_io_convert::from::FromFutures<Pin<Box<dyn ARWS>>>);
// impl Target{
//     pub async fn get(&mut self,p: Packet) -> anyhow::Result<Vec<u8>>{
//         p.encode(&mut self.0).await?;
//         let v = decode_vec(&mut self.0).await?;
//         return Ok(v);
//     }
// }
pub struct Processor<H> {
    pub me: Id,
    pub handler: H,
    pub all: VecDeque<Box<dyn Pusher + Send>>,
}
impl<H: Handler> Processor<H> {
    pub async fn process(&mut self, mut p: Packet) -> anyhow::Result<Vec<u8>> {
        let tm = x25519_base(self.me);
        if p.taint.contains(&tm) {
            return Ok(vec![]);
        }
        p.taint.insert(tm);
        if tm != p.target {
            let mut v = vec![];
            while v == vec![] {
                // match self.all.front_mut().unwrap().push(p.clone()).await {
                //     Ok(w) => {
                //         v = w;
                //         if v == vec![] {
                //             let p = self.all.pop_front().unwrap();
                //             self.all.push_back(p);
                //         }
                //     }
                //     Err(_) => {
                //         self.all.pop_front();
                //     }
                // }
                (v, _) = futures::future::select_ok(
                    self.all
                        .iter_mut()
                        .map(|a| Box::pin(async { a.push(p.clone()).await })),
                )
                .await?;
                if v == vec![] {
                    let p = self.all.pop_front().unwrap();
                    self.all.push_back(p);
                }
            }
            return Ok(v);
        }
        let d = simple_encryption::decrypt(&self.me, &p.data)?;
        let dk = d[0..32].try_into()?;
        let d = d[32..].to_vec();
        let mut h = self.handler.handle(d, &mut self.all).await?;
        h = simple_encryption::encrypt(&dk, &h)?;
        h.push(0);
        return Ok(h);
    }
}
#[async_trait::async_trait]
impl<H: Handler + Send> Pusher for Processor<H> {
    async fn push(&mut self, a: Packet) -> anyhow::Result<Vec<u8>> {
        return self.process(a).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_a() {
        struct Echo {}
        #[async_trait::async_trait]
        impl Handler for Echo {
            async fn handle(
                &mut self,
                p: Vec<u8>,
                a: &mut VecDeque<Box<dyn Pusher + Send>>,
            ) -> anyhow::Result<Vec<u8>> {
                return Ok(p);
            }
        }
        let k = rand::random();
        let mut f = Processor {
            me: k,
            handler: Echo {},
            all: VecDeque::new(),
        };
        // let ft = ;
        let _ = pasts::Executor::default().block_on(async move {
            assert_eq!(
                f.push_encrypted(x25519_base(k), vec![0, 1, 2])
                    .await
                    .unwrap(),
                vec![0, 1, 2]
            );
        });
        // drop(ft);
    }
}
