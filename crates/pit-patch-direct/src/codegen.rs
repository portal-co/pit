use wasm_encoder::{Encode, Instruction, ValType};

/// A stack-machine wasm function builder for generated (non-SSA) function bodies.
pub struct FuncBuilder {
    pub type_index: u32,
    /// Collected local groups: (count, ValType).
    locals: Vec<(u32, ValType)>,
    /// Encoded instruction bytes (accumulated via `Instruction::encode`).
    insns: Vec<u8>,
}

impl FuncBuilder {
    pub fn new(type_index: u32) -> Self {
        FuncBuilder {
            type_index,
            locals: Vec::new(),
            insns: Vec::new(),
        }
    }

    /// Add a local group (count, type); locals are indexed after function params.
    pub fn add_local_group(&mut self, count: u32, ty: ValType) -> &mut Self {
        self.locals.push((count, ty));
        self
    }

    /// Emit a single instruction (Instruction<'_> implements wasm_encoder::Encode).
    pub fn emit(&mut self, insn: Instruction<'_>) -> &mut Self {
        insn.encode(&mut self.insns);
        self
    }

    /// Finish the function.  Appends `end` and returns `(locals, body_bytes)` for
    /// use with `DirectModule::add_func`.
    pub fn finish(mut self) -> (Vec<(u32, ValType)>, Vec<u8>) {
        Instruction::End.encode(&mut self.insns);
        (self.locals, self.insns)
    }
}
