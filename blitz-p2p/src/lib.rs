use std::{collections::BTreeSet, sync::Arc};
use futures::{lock::Mutex, AsyncRead, AsyncWrite};
pub type Id = [u8; 32];
pub mod packet;
pub mod socket;
pub fn arc_mutex<T>(v: T) -> Arc<Mutex<T>>{
    return Arc::new(Mutex::new(v));
}
#[cfg(test)]
mod tests {
    use super::*;

}
