use std::collections::BTreeMap;

use once_cell::sync::Lazy;

static mut MEMS: Lazy<BTreeMap<usize,Vec<[u8;65536]>>> = Lazy::new(||BTreeMap::new());
unsafe fn mem(a: usize) -> &'static mut Vec<[u8;65536]>{
    return MEMS.entry(a).or_insert_with(||vec![]);
}
#[export_name = "sk%resolve"]
unsafe extern "C" fn resolve(x: usize,m: usize) -> *mut u8{
    return (mem(m).as_mut_ptr() as *mut u8).add(x);
}
#[export_name = "sk%grow"]
unsafe extern "C" fn grow(n: usize,m: usize){
    for _ in 0..n{
        mem(m).push([0u8; 65536])
    }
}
#[export_name = "sk%size"]
unsafe extern "C" fn size(m: usize) -> usize{
    return mem(m).len();
}
#[cfg(test)]
mod tests {
    use super::*;


}
