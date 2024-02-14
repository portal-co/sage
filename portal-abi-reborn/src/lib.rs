use iced_x86::{code_asm::*, Register};
pub fn setup_stack(c: &mut CodeAssembler) -> Result<(), IcedError> {
    c.push(r15)?;
    c.mov(r15, rsp)?;
    return Ok(());
}
pub fn finish_stack(c: &mut CodeAssembler) -> Result<(), IcedError> {
    c.mov(rsp, r15)?;
    c.pop(r15)?;
    return Ok(());
}
pub fn switch_guest_host(c: &mut CodeAssembler) -> Result<(),IcedError>{
    c.xchg(rsp, r15)?;
    return Ok(());
}
//CLOBBERS r13
pub fn pop_parent(c: &mut CodeAssembler) -> Result<(),IcedError>{
    // c.push(rax)?;
    c.push(r14)?;
    switch_guest_host(c)?;
    c.pop(r14)?;
    c.pop(r13)?;
    c.push(r14)?;
    switch_guest_host(c)?;
    c.pop(r14)?;
    c.push(r13)?;
    return Ok(());
}
pub fn push_parent(c: &mut CodeAssembler) -> Result<(),IcedError>{
    c.pop(r13)?;
    c.push(r14)?;
    switch_guest_host(c)?;
    c.pop(r14)?;
    c.push(r13)?;
    c.push(r14)?;
    switch_guest_host(c)?;
    c.pop(r14)?;
    return Ok(());
}
#[cfg(test)]
mod tests {
    use super::*;
}
