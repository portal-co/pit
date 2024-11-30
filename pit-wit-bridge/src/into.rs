use std::{
    collections::{BTreeMap, BTreeSet},
    mem::{replace, swap, take},
};

use nonempty::NonEmpty;
use pit_core::{Arg, Interface, Sig};
use waffle::{
    util::new_sig, Block, ExportKind, Func, FuncDecl, FunctionBody, Memory, Module, Operator, Signature, SignatureData, Type, Value, WithNullable
};
use wit_bindgen_core::{
    abi::{Bindgen, Bitcast, WasmType},
    wit_parser::{self, Record, Resolve, Tuple},
};

pub struct State<'a, 'b, 'c> {
    pub module: &'a mut Module<'b>,
    pub func: &'c mut FunctionBody,
    pub scratch: Memory,
    pub block_stack: NonEmpty<Block>,
    pub ids: Vec<Func>,
    pub storage: BTreeSet<Interface>,
}

impl<'a, 'b, 'c> State<'a, 'b, 'c> {
    pub fn waffle(&self, a: &WasmType) -> Type {
        let p = if self.module.memories[self.scratch].memory64 {
            Type::I64
        } else {
            Type::I32
        };
        match a {
            WasmType::I32 => Type::I32,
            WasmType::I64 => Type::I64,
            WasmType::F32 => Type::F32,
            WasmType::F64 => Type::F64,
            WasmType::Pointer => p,
            WasmType::PointerOrI64 => p,
            WasmType::Length => p,
        }
    }
    pub fn ty(&mut self, t: &wit_parser::Type, resolve: &Resolve) -> Arg {
        match t {
            wit_parser::Type::Bool => Arg::I32,
            wit_parser::Type::U8 => Arg::I32,
            wit_parser::Type::U16 => Arg::I32,
            wit_parser::Type::U32 => Arg::I32,
            wit_parser::Type::U64 => Arg::I64,
            wit_parser::Type::S8 => Arg::I32,
            wit_parser::Type::S16 => Arg::I32,
            wit_parser::Type::S32 => Arg::I32,
            wit_parser::Type::S64 => Arg::I64,
            wit_parser::Type::F32 => Arg::F32,
            wit_parser::Type::F64 => Arg::F64,
            wit_parser::Type::Char => Arg::I32,
            wit_parser::Type::String => todo!(),
            wit_parser::Type::Id(id) => {
                let def: &wit_parser::TypeDef = &resolve.types[*id];
                match &def.kind {
                    wit_parser::TypeDefKind::Record(record) => Arg::Resource {
                        ty: pit_core::ResTy::Of(self.record(record, resolve).rid()),
                        nullable: true,
                        take: false,
                        ann: vec![],
                    },
                    wit_parser::TypeDefKind::Resource => Arg::Resource {
                        ty: pit_core::ResTy::None,
                        nullable: true,
                        take: false,
                        ann: vec![],
                    },
                    wit_parser::TypeDefKind::Handle(handle) => todo!(),
                    wit_parser::TypeDefKind::Flags(flags) => todo!(),
                    wit_parser::TypeDefKind::Tuple(tuple) => Arg::Resource {
                        ty: pit_core::ResTy::Of(self.tuple(tuple, resolve).rid()),
                        nullable: true,
                        take: false,
                        ann: vec![],
                    },
                    wit_parser::TypeDefKind::Variant(variant) => todo!(),
                    wit_parser::TypeDefKind::Enum(_) => todo!(),
                    wit_parser::TypeDefKind::Option(_) => todo!(),
                    wit_parser::TypeDefKind::Result(result) => todo!(),
                    wit_parser::TypeDefKind::List(_) => todo!(),
                    wit_parser::TypeDefKind::Future(_) => todo!(),
                    wit_parser::TypeDefKind::Stream(stream) => todo!(),
                    wit_parser::TypeDefKind::Type(_) => todo!(),
                    wit_parser::TypeDefKind::Unknown => todo!(),
                }
            }
        }
    }
    pub fn record(&mut self, r: &Record, resolve: &Resolve) -> Interface {
        let i = Interface {
            methods: r
                .fields
                .iter()
                .map(|f: &wit_bindgen_core::wit_parser::Field| {
                    (
                        f.name.clone(),
                        Sig {
                            ann: vec![],
                            params: vec![],
                            rets: vec![self.ty(&f.ty, resolve)],
                        },
                    )
                })
                .collect(),
            ann: vec![],
        };
        self.storage.insert(i.clone());
        return i;
    }
    pub fn tuple(&mut self, r: &Tuple, resolve: &Resolve) -> Interface {
        let i = Interface {
            methods: r
                .types
                .iter()
                .enumerate()
                .map(|(v, f)| {
                    (
                        format!("v{v}"),
                        Sig {
                            ann: vec![],
                            params: vec![],
                            rets: vec![self.ty(&f, resolve)],
                        },
                    )
                })
                .collect(),
            ann: vec![],
        };
        self.storage.insert(i.clone());

        return i;
    }
    fn lfn(&self, n: &str) -> Option<Func> {
        return self.module.exports.iter().find_map(|a| {
            if a.name == n {
                match &a.kind {
                    ExportKind::Func(f) => Some(*f),
                    _ => None,
                }
            } else {
                None
            }
        });
    }
    fn bitcast(&mut self, o: Value, c: &Bitcast) -> Value {
        let k = self.block_stack.last().clone();
        match c {
            Bitcast::F32ToI32 => {
                self.func
                    .add_op(k, Operator::I32ReinterpretF32, &[o], &[Type::I32])
            }
            Bitcast::F64ToI64 => {
                self.func
                    .add_op(k, Operator::I64ReinterpretF64, &[o], &[Type::I64])
            }
            Bitcast::I32ToI64 => self
                .func
                .add_op(k, Operator::I64ExtendI32U, &[o], &[Type::I64]),
            Bitcast::F32ToI64 => {
                let o = self.bitcast(o, &Bitcast::F32ToI32);
                self.bitcast(o, &Bitcast::I32ToI64)
            }
            Bitcast::I32ToF32 => {
                self.func
                    .add_op(k, Operator::F32ReinterpretI32, &[o], &[Type::F32])
            }
            Bitcast::I64ToF64 => {
                self.func
                    .add_op(k, Operator::F64ReinterpretI64, &[o], &[Type::F64])
            }
            Bitcast::I64ToI32 => self
                .func
                .add_op(k, Operator::I32WrapI64, &[o], &[Type::I32]),
            Bitcast::I64ToF32 => {
                let o = self.bitcast(o, &Bitcast::I64ToI32);
                self.bitcast(o, &Bitcast::I32ToF32)
            }
            Bitcast::P64ToI64 => o,
            Bitcast::I64ToP64 => o,
            Bitcast::P64ToP | Bitcast::I64ToL => {
                if self.module.memories[self.scratch].memory64 {
                    o
                } else {
                    self.bitcast(o, &Bitcast::I64ToI32)
                }
            }
            Bitcast::PToP64 | Bitcast::LToI64 => {
                if self.module.memories[self.scratch].memory64 {
                    o
                } else {
                    self.bitcast(o, &Bitcast::I32ToI64)
                }
            }
            Bitcast::I32ToP | Bitcast::I32ToL => {
                if self.module.memories[self.scratch].memory64 {
                    self.bitcast(o, &Bitcast::I32ToI64)
                } else {
                    o
                }
            }
            Bitcast::PToI32 | Bitcast::LToI32 => {
                if self.module.memories[self.scratch].memory64 {
                    self.bitcast(o, &Bitcast::I64ToI32)
                } else {
                    o
                }
            }
            Bitcast::PToL => o,
            Bitcast::LToP => o,
            Bitcast::Sequence(a) => {
                let o = self.bitcast(o, &a[0]);
                self.bitcast(o, &a[1])
            }
            Bitcast::None => o,
        }
    }
    fn pop_block(&mut self) -> Func {
        let sig = new_sig(
            self.module,
            SignatureData::Func {
                params: vec![Type::Heap(WithNullable { value: waffle::HeapType::ExternRef, nullable: true })],
                returns: self.func.rets.clone(),
            },
        );
        let id = self.ids.pop().unwrap();
        swap(self.module.funcs[id].body_mut().unwrap(), &mut self.func);
        if let FuncDecl::Body(s, _, _) = &mut self.module.funcs[id] {
            *s = sig;
        }
        return id;
    }
}
impl<'a, 'b, 'c> Bindgen for State<'a, 'b, 'c> {
    type Operand = Value;

    fn emit(
        &mut self,
        resolve: &wit_bindgen_core::wit_parser::Resolve,
        inst: &wit_bindgen_core::abi::Instruction<'_>,
        operands: &mut Vec<Self::Operand>,
        results: &mut Vec<Self::Operand>,
    ) {
        let k = self.block_stack.last().clone();
        match inst {
            wit_bindgen_core::abi::Instruction::GetArg { nth } => {
                results.push(self.func.blocks[self.func.entry].params[*nth].1);
            }
            wit_bindgen_core::abi::Instruction::I32Const { val } => results.push(self.func.add_op(
                k,
                waffle::Operator::I32Const { value: *val as u32 },
                &[],
                &[Type::I32],
            )),
            wit_bindgen_core::abi::Instruction::Bitcasts { casts } => {
                results.extend(
                    operands
                        .drain(..)
                        .zip(casts.iter())
                        .map(|(o, c)| self.bitcast(o, c)),
                );
            }
            wit_bindgen_core::abi::Instruction::ConstZero { tys } => {
                results.extend(tys.iter().map(|a| {
                    let w = self.waffle(a);
                    match &w {
                        Type::I32 => {
                            self.func
                                .add_op(k, waffle::Operator::I32Const { value: 0 }, &[], &[w])
                        }
                        Type::I64 => {
                            self.func
                                .add_op(k, waffle::Operator::I64Const { value: 0 }, &[], &[w])
                        }
                        Type::F32 => {
                            self.func
                                .add_op(k, waffle::Operator::F32Const { value: 0 }, &[], &[w])
                        }
                        Type::F64 => {
                            self.func
                                .add_op(k, waffle::Operator::F64Const { value: 0 }, &[], &[w])
                        }
                        _ => unreachable!(),
                    }
                }));
            }
            wit_bindgen_core::abi::Instruction::I32Load { offset } => {
                results.push(self.func.add_op(
                    k,
                    Operator::I32Load {
                        memory: waffle::MemoryArg {
                            align: 2,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[operands.pop().unwrap()],
                    &[Type::I32],
                ));
            }
            wit_bindgen_core::abi::Instruction::I32Load8U { offset } => {
                results.push(self.func.add_op(
                    k,
                    Operator::I32Load8U {
                        memory: waffle::MemoryArg {
                            align: 0,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[operands.pop().unwrap()],
                    &[Type::I32],
                ));
            }
            wit_bindgen_core::abi::Instruction::I32Load8S { offset } => {
                results.push(self.func.add_op(
                    k,
                    Operator::I32Load8S {
                        memory: waffle::MemoryArg {
                            align: 0,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[operands.pop().unwrap()],
                    &[Type::I32],
                ));
            }
            wit_bindgen_core::abi::Instruction::I32Load16U { offset } => {
                results.push(self.func.add_op(
                    k,
                    Operator::I32Load16U {
                        memory: waffle::MemoryArg {
                            align: 1,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[operands.pop().unwrap()],
                    &[Type::I32],
                ));
            }
            wit_bindgen_core::abi::Instruction::I32Load16S { offset } => {
                results.push(self.func.add_op(
                    k,
                    Operator::I32Load16S {
                        memory: waffle::MemoryArg {
                            align: 1,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[operands.pop().unwrap()],
                    &[Type::I32],
                ));
            }
            wit_bindgen_core::abi::Instruction::I64Load { offset } => {
                results.push(self.func.add_op(
                    k,
                    Operator::I64Load {
                        memory: waffle::MemoryArg {
                            align: 3,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[operands.pop().unwrap()],
                    &[Type::I64],
                ));
            }
            wit_bindgen_core::abi::Instruction::F32Load { offset } => {
                results.push(self.func.add_op(
                    k,
                    Operator::F32Load {
                        memory: waffle::MemoryArg {
                            align: 2,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[operands.pop().unwrap()],
                    &[Type::F32],
                ));
            }
            wit_bindgen_core::abi::Instruction::F64Load { offset } => {
                results.push(self.func.add_op(
                    k,
                    Operator::F64Load {
                        memory: waffle::MemoryArg {
                            align: 3,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[operands.pop().unwrap()],
                    &[Type::F64],
                ));
            }
            wit_bindgen_core::abi::Instruction::PointerLoad { offset }
            | wit_bindgen_core::abi::Instruction::LengthLoad { offset } => {
                let (op, ty) = if self.module.memories[self.scratch].memory64 {
                    (
                        Operator::I64Load {
                            memory: waffle::MemoryArg {
                                align: 3,
                                offset: *offset as u64,
                                memory: self.scratch,
                            },
                        },
                        Type::I64,
                    )
                } else {
                    (
                        Operator::I32Load {
                            memory: waffle::MemoryArg {
                                align: 2,
                                offset: *offset as u64,
                                memory: self.scratch,
                            },
                        },
                        Type::I32,
                    )
                };
                results.push(self.func.add_op(k, op, &[operands.pop().unwrap()], &[ty]));
            }
            wit_bindgen_core::abi::Instruction::I32Store { offset } => {
                let ptr = operands.pop().unwrap();
                let val = operands.pop().unwrap();
                self.func.add_op(
                    k,
                    Operator::I32Store {
                        memory: waffle::MemoryArg {
                            align: 2,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[ptr, val],
                    &[],
                );
            }
            wit_bindgen_core::abi::Instruction::I32Store8 { offset } => {
                let ptr = operands.pop().unwrap();
                let val = operands.pop().unwrap();
                self.func.add_op(
                    k,
                    Operator::I32Store8 {
                        memory: waffle::MemoryArg {
                            align: 0,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[ptr, val],
                    &[],
                );
            }
            wit_bindgen_core::abi::Instruction::I32Store16 { offset } => {
                let ptr = operands.pop().unwrap();
                let val = operands.pop().unwrap();
                self.func.add_op(
                    k,
                    Operator::I32Store16 {
                        memory: waffle::MemoryArg {
                            align: 1,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[ptr, val],
                    &[],
                );
            }
            wit_bindgen_core::abi::Instruction::I64Store { offset } => {
                let ptr = operands.pop().unwrap();
                let val = operands.pop().unwrap();
                self.func.add_op(
                    k,
                    Operator::I64Store {
                        memory: waffle::MemoryArg {
                            align: 3,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[ptr, val],
                    &[],
                );
            }
            wit_bindgen_core::abi::Instruction::F32Store { offset } => {
                let ptr = operands.pop().unwrap();
                let val = operands.pop().unwrap();
                self.func.add_op(
                    k,
                    Operator::F32Store {
                        memory: waffle::MemoryArg {
                            align: 2,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[ptr, val],
                    &[],
                );
            }
            wit_bindgen_core::abi::Instruction::F64Store { offset } => {
                let ptr = operands.pop().unwrap();
                let val = operands.pop().unwrap();
                self.func.add_op(
                    k,
                    Operator::F64Store {
                        memory: waffle::MemoryArg {
                            align: 3,
                            offset: *offset as u64,
                            memory: self.scratch,
                        },
                    },
                    &[ptr, val],
                    &[],
                );
            }
            wit_bindgen_core::abi::Instruction::PointerStore { offset }
            | wit_bindgen_core::abi::Instruction::LengthStore { offset } => {
                let ptr = operands.pop().unwrap();
                let val = operands.pop().unwrap();
                self.func.add_op(
                    k,
                    if self.module.memories[self.scratch].memory64 {
                        Operator::I64Store {
                            memory: waffle::MemoryArg {
                                align: 3,
                                offset: *offset as u64,
                                memory: self.scratch,
                            },
                        }
                    } else {
                        Operator::I32Store {
                            memory: waffle::MemoryArg {
                                align: 2,
                                offset: *offset as u64,
                                memory: self.scratch,
                            },
                        }
                    },
                    &[ptr, val],
                    &[],
                );
            }
            wit_bindgen_core::abi::Instruction::I32FromChar => swap(operands, results),
            wit_bindgen_core::abi::Instruction::I64FromU64 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::I64FromS64 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::I32FromU32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::I32FromS32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::I32FromU16 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::I32FromS16 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::I32FromU8 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::I32FromS8 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::CoreF32FromF32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::CoreF64FromF64 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::S8FromI32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::U8FromI32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::S16FromI32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::U16FromI32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::S32FromI32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::U32FromI32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::S64FromI64 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::U64FromI64 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::CharFromI32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::F32FromCoreF32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::F64FromCoreF64 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::BoolFromI32 => swap(operands, results),
            wit_bindgen_core::abi::Instruction::I32FromBool => swap(operands, results),
            wit_bindgen_core::abi::Instruction::ListCanonLower { element, realloc } => todo!(),
            wit_bindgen_core::abi::Instruction::StringLower { realloc } => todo!(),
            wit_bindgen_core::abi::Instruction::ListLower { element, realloc } => todo!(),
            wit_bindgen_core::abi::Instruction::ListCanonLift { element, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::StringLift => todo!(),
            wit_bindgen_core::abi::Instruction::ListLift { element, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::IterElem { element } => todo!(),
            wit_bindgen_core::abi::Instruction::IterBasePointer => todo!(),
            wit_bindgen_core::abi::Instruction::RecordLower { record, name, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::RecordLift { record, name, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::HandleLower { handle, name, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::HandleLift { handle, name, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::TupleLower { tuple, ty } => {
                let t = self.tuple(&**tuple, resolve);
                let funcs = pit_patch::util::waffle_funcs(&mut self.module, &t, false);
                let funcs = tuple
                    .types
                    .iter()
                    .enumerate()
                    .filter_map(|(v, _)| funcs.get(&format!("v{v}")))
                    .cloned()
                    .collect::<Vec<_>>();
                let val = operands.pop().unwrap();
                results.extend(funcs.into_iter().map(|f| {
                    let tys = self.module.funcs[f].sig();
                    let SignatureData::Func { params, returns } = &self.module.signatures[tys] else{
                        todo!()
                    };
                    let tys = &*returns;
                    let v = self
                        .func
                        .add_op(k, Operator::Call { function_index: f }, &[val], tys);
                    v
                }));
            }
            wit_bindgen_core::abi::Instruction::TupleLift { tuple, ty } => {
                let t = self.tuple(&**tuple, resolve);
                let c = pit_patch::util::canon(&mut self.module, &t,None,"canon");
                results.push(self.func.add_op(
                    k,
                    Operator::Call { function_index: c },
                    &operands,
                    &[Type::Heap(WithNullable{nullable: true, value: waffle::HeapType::ExternRef})],
                ));
            }
            wit_bindgen_core::abi::Instruction::FlagsLower { flags, name, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::FlagsLift { flags, name, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::VariantPayloadName => todo!(),
            wit_bindgen_core::abi::Instruction::VariantLower {
                variant,
                name,
                ty,
                results,
            } => todo!(),
            wit_bindgen_core::abi::Instruction::VariantLift { variant, name, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::EnumLower { enum_, name, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::EnumLift { enum_, name, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::OptionLower {
                payload,
                ty,
                results,
            } => todo!(),
            wit_bindgen_core::abi::Instruction::OptionLift { payload, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::ResultLower {
                result,
                ty,
                results,
            } => todo!(),
            wit_bindgen_core::abi::Instruction::ResultLift { result, ty } => todo!(),
            wit_bindgen_core::abi::Instruction::CallWasm { name, sig } => todo!(),
            wit_bindgen_core::abi::Instruction::CallInterface { func } => todo!(),
            wit_bindgen_core::abi::Instruction::Return { amt, func } => {
                self.func.set_terminator(
                    k,
                    waffle::Terminator::Return {
                        values: take(operands),
                    },
                );
                *self.block_stack.last_mut() = self.func.add_block();
            }
            wit_bindgen_core::abi::Instruction::Malloc {
                realloc,
                size,
                align,
            } => todo!(),
            wit_bindgen_core::abi::Instruction::GuestDeallocate { size, align } => todo!(),
            wit_bindgen_core::abi::Instruction::GuestDeallocateString => todo!(),
            wit_bindgen_core::abi::Instruction::GuestDeallocateList { element } => todo!(),
            wit_bindgen_core::abi::Instruction::GuestDeallocateVariant { blocks } => todo!(),
        }
    }

    fn return_pointer(&mut self, size: usize, align: usize) -> Self::Operand {
        todo!()
    }

    fn push_block(&mut self) {
        let sig = new_sig(
            self.module,
            SignatureData::Func {
                params: vec![Type::Heap(WithNullable{nullable: true, value: waffle::HeapType::ExternRef})],
                returns: vec![],
            },
        );
        let mut new = FunctionBody::new(&self.module, sig);
        self.block_stack.push(new.entry);
        new = replace(&mut self.func, new);
        self.ids.push(self.module.funcs.push(waffle::FuncDecl::Body(
            sig,
            format!("component/synthetic"),
            new,
        )));
    }

    fn finish_block(&mut self, operand: &mut Vec<Self::Operand>) {
        self.func.rets = operand
            .iter()
            .filter_map(|a| self.func.values[*a].ty(&self.func.type_pool))
            .collect();
        self.func.set_terminator(
            *self.block_stack.last(),
            waffle::Terminator::Return {
                values: take(operand),
            },
        );
    }

    fn sizes(&self) -> &wit_bindgen_core::wit_parser::SizeAlign {
        todo!()
    }

    fn is_list_canonical(
        &self,
        resolve: &wit_bindgen_core::wit_parser::Resolve,
        element: &wit_bindgen_core::wit_parser::Type,
    ) -> bool {
        todo!()
    }
}
