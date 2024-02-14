use std::collections::BTreeMap;

use waffle::ImportKind;

#[async_trait::async_trait]
pub trait Fetcher{
    async fn fetch(&self, a: String) -> anyhow::Result<waffle::Module<'static>>;
}

pub struct FetchManager{
    pub cache: BTreeMap<String,BTreeMap<String,ImportKind>>
}

#[cfg(test)]
mod tests {
    use super::*;


}
