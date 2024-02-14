use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use anyhow::Context;
use id_arena::Id;
use indexmap::IndexMap;
use waffle::{
    entity::EntityRef, more::new_sig, op_traits::op_outputs, Block, BlockTarget, Func, FuncDecl, FunctionBody, Operator, SignatureData, Value, ValueDef
};

use crate::{Call, Pick, Target};

use super::import::{WaffleTerm};
pub trait WaffleOp<O, T, D, S> {
    fn to_op<
        A: Deref<Target = waffle::Module<'static>> + DerefMut,
        B: Deref<Target = crate::Module<O, T, D, S>>,
    >(
        &self,
        ctx: &mut ToWaffle<A, B, O, T, D, S>,
        f: &mut FunctionBody,
        args: &[Value],
    ) -> anyhow::Result<waffle::Value>;
}
impl<O,T,D,S> WaffleOp<O,T,D,S> for Operator{
    fn to_op<
        A: Deref<Target = waffle::Module<'static>> + DerefMut,
        B: Deref<Target = crate::Module<O, T, D, S>>,
    >(
        &self,
        ctx: &mut ToWaffle<A, B, O, T, D, S>,
        gb: &mut FunctionBody,
        r: &[Value],
    ) -> anyhow::Result<waffle::Value> {
        let mut rt = vec![];
        for s in r{
            rt.push((gb.values[*s].ty(&gb.type_pool).unwrap(),*s))
        }
        let r = gb.arg_pool.from_iter(r.iter().map(|a|*a));
        let ts = op_outputs(&ctx.waffle, Some(&rt), self)?;
        let ts = gb.type_pool.from_iter(ts.iter().map(|a|*a));
        return Ok(gb.add_value(ValueDef::Operator(self.clone(), r, ts)));
    }
}
impl<O,T,D,S> WaffleOp<O,T,D,S> for Pick{
    fn to_op<
        A: Deref<Target = waffle::Module<'static>> + DerefMut,
        B: Deref<Target = crate::Module<O, T, D, S>>,
    >(
        &self,
        ctx: &mut ToWaffle<A, B, O, T, D, S>,
        f: &mut FunctionBody,
        args: &[Value],
    ) -> anyhow::Result<waffle::Value> {
        let t = f.values[args[0]].tys(&f.type_pool)[self.index];
        return Ok(f.add_value(ValueDef::PickOutput(args[0], self.index as u32, t)));
    }
}
impl<O: WaffleOp<O, T, D, S> + Clone,T: Into<WaffleTerm<O, T, D, S>> + Clone,D,S: Into<SignatureData> + Clone> WaffleOp<O,T,D,S> for Call<O,T,D,S>{
    fn to_op<
        A: Deref<Target = waffle::Module<'static>> + DerefMut,
        B: Deref<Target = crate::Module<O, T, D, S>>,
    >(
        &self,
        ctx: &mut ToWaffle<A, B, O, T, D, S>,
        f: &mut FunctionBody,
        args: &[Value],
    ) -> anyhow::Result<waffle::Value> {
        let cf = ctx.translate(self.func)?;
        return Operator::Call { function_index: cf }.to_op(ctx, f, args);
    }
}
pub struct ToWaffle<A, B, O, T, D, S> {
    pub waffle: A,
    pub axle: B,
    pub fcache: BTreeMap<Id<crate::Func<O, T, D, S>>, Func>,
}
impl<
        A: Deref<Target = waffle::Module<'static>> + DerefMut,
        B: Deref<Target = crate::Module<O, T, D, S>>,
        O: WaffleOp<O, T, D, S> + Clone,
        T: Into<WaffleTerm<O, T, D, S>> + Clone,
        D,
        S: Into<SignatureData> + Clone,
    > ToWaffle<A, B, O, T, D, S>
{
    pub fn translate(&mut self, f: Id<crate::Func<O, T, D, S>>) -> anyhow::Result<Func> {
        loop {
            if let Some(g) = self.fcache.get(&f) {
                return Ok(*g);
            }
            let g = self.waffle.funcs.push(FuncDecl::None);
            let fb = self.axle.funcs[f].clone();
            let sig = new_sig(&mut self.waffle, fb.sig.into());
            let mut gb = FunctionBody::new(&self.waffle, sig);
            let mut m = BTreeMap::new();
            for (n,v) in fb.values.iter(){
                let value = match v{
                    crate::Value::Operator(o, s, _) => {
                        let mut r = vec![];
                        for a in s{
                            r.push(*m.get(a).context("in getting a value")?)
                        };
                        let o = o.to_op(self,&mut gb,&r)?;
                        gb.append_to_block(gb.entry, o);
                        ValueDef::Alias(o)
                    },
                    crate::Value::Param(p) => waffle::ValueDef::Alias(gb.blocks[gb.entry].params[*p].1),
                    crate::Value::Alias(s) => waffle::ValueDef::Alias(*m.get(s).context("in getting a value")?),
                };
                let value = gb.add_value(value);
                gb.append_to_block(gb.entry, value);
                m.insert(n.clone(), value);
            }
            match Into::<WaffleTerm<O,T,D,S>>::into(fb.terminator){
                WaffleTerm::Br(t) => {
                    let tt = self.translate(t.id)?;
                    let mut r = vec![];
                    for a in t.args{
                        r.push(*m.get(&a).context("in getting a value")?)
                    };
                    gb.set_terminator(gb.entry, waffle::Terminator::ReturnCall { func: tt, args: r });
                },
                WaffleTerm::BrIf { cond, then, els } => {
                    let if_true = gb.add_block();
                    let if_false = gb.add_block();
                    for (t,block) in vec![(then,if_true),(els,if_false)]{
                        let tt = self.translate(t.id)?;
                        let mut r = vec![];
                        for a in t.args{
                            r.push(*m.get(&a).context("in getting a value")?)
                        };
                        gb.set_terminator(block, waffle::Terminator::ReturnCall { func: tt, args: r });
                    }
                    gb.set_terminator(gb.entry, waffle::Terminator::CondBr { cond: *m.get(&cond).context("in getting a value")?, if_true: BlockTarget{block: if_true, args: vec![]}, if_false: BlockTarget{block: if_false, args: vec![]} })
                },
                WaffleTerm::Return(s) => {
                    let mut r = vec![];
                    for a in s{
                        r.push(*m.get(&a).context("in getting a value")?)
                    };
                    gb.set_terminator(gb.entry, waffle::Terminator::Return { values: r })
                },
                WaffleTerm::None => {
                    gb.set_terminator(gb.entry, waffle::Terminator::Unreachable);
                },
            }
            self.waffle.funcs[g] = FuncDecl::Body(sig, format!("{f:?}"), gb);
            self.fcache.insert(f, g);
        }
    }
}
