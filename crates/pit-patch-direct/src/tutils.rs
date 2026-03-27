use wasm_encoder::{BlockType, Instruction, ValType, RefType};
use crate::module::DirectModule;
use crate::codegen::FuncBuilder;

/// Generate a `talloc` function for the given table index.
///
/// Signature: `(val: <table-element-type>, aux_vals...) -> i32`
///
/// Finds the first null slot in table `t`, writes `val` there, and returns the
/// index.  If the table is full, grows it by 1.  Auxiliary tables are grown in
/// parallel to keep their sizes in sync.
///
/// Returns the new func index.
pub fn talloc(m: &mut DirectModule, t: u32, aux: &[u32]) -> anyhow::Result<u32> {
    let elem_ty = m.tables[t as usize - m.imports.iter().filter(|i| matches!(i.ty, crate::module::DirectImportKind::Table(_))).count()].ty.element_type;
    let aux_elem_tys: Vec<_> = aux.iter().map(|&a| table_elem_vt(m, a)).collect();

    // Params: (val: elem_ty, aux_0: aux_elem_tys[0], ...)
    // Returns: i32
    let mut params: Vec<ValType> = vec![vt_of_ref(elem_ty)];
    for aty in &aux_elem_tys {
        params.push(*aty);
    }
    let ti = m.add_func_type(&params, &[ValType::I32]);

    // Locals:
    //   params: 0 = val, 1..=aux.len() = aux vals
    //   local 1+aux.len() = idx (i32)
    let idx_local = 1 + aux.len() as u32;

    let mut b = FuncBuilder::new(ti);
    b.add_local_group(1, ValType::I32); // idx

    // i32.const 0; local.set $idx
    b.emit(Instruction::I32Const(0));
    b.emit(Instruction::LocalSet(idx_local));

    // loop $search
    b.emit(Instruction::Loop(BlockType::Empty));

    //   local.get $idx; table.get $t; ref.is_null
    b.emit(Instruction::LocalGet(idx_local));
    b.emit(Instruction::TableGet(t));
    b.emit(Instruction::RefIsNull);

    //   if  -- found a null slot
    b.emit(Instruction::If(BlockType::Empty));
    //     write val into slot
    b.emit(Instruction::LocalGet(idx_local));
    b.emit(Instruction::LocalGet(0)); // val
    b.emit(Instruction::TableSet(t));
    //     write aux vals
    for (i, &a) in aux.iter().enumerate() {
        let size_ok_block_depth = 0u32; // we'll handle aux growth separately
        b.emit(Instruction::LocalGet(idx_local));
        b.emit(Instruction::LocalGet(1 + i as u32));
        b.emit(Instruction::TableSet(a));
    }
    b.emit(Instruction::LocalGet(idx_local));
    b.emit(Instruction::Return);
    b.emit(Instruction::End); // end if

    //   increment idx
    b.emit(Instruction::LocalGet(idx_local));
    b.emit(Instruction::I32Const(1));
    b.emit(Instruction::I32Add);
    b.emit(Instruction::LocalTee(idx_local));

    //   compare to table size
    b.emit(Instruction::TableSize(t));
    b.emit(Instruction::I32GeU);

    //   if at end — grow
    b.emit(Instruction::If(BlockType::Empty));
    //     grow table by 1 with val
    b.emit(Instruction::LocalGet(0)); // val
    b.emit(Instruction::I32Const(1));
    b.emit(Instruction::TableGrow(t));
    b.emit(Instruction::Drop); // discard old size (we already have idx)
    //     grow aux tables
    for (i, &a) in aux.iter().enumerate() {
        b.emit(Instruction::LocalGet(1 + i as u32));
        b.emit(Instruction::I32Const(1));
        b.emit(Instruction::TableGrow(a));
        b.emit(Instruction::Drop);
    }
    b.emit(Instruction::LocalGet(idx_local));
    b.emit(Instruction::Return);
    b.emit(Instruction::End); // end if-grow

    //   br to loop
    b.emit(Instruction::Br(0));
    b.emit(Instruction::End); // end loop

    b.emit(Instruction::Unreachable);

    let (locals, body) = b.finish();
    Ok(m.add_func(ti, locals, body))
}

/// Generate a `tfree` function for the given table index.
///
/// Signature: `(idx: i32) -> (val: <table-element-type>, aux_vals...)`
///
/// Reads the value at `idx`, clears the slot to null, and returns the value
/// (plus aux table values).
///
/// Returns the new func index.
pub fn tfree(m: &mut DirectModule, t: u32, aux: &[u32]) -> anyhow::Result<u32> {
    let elem_ty = table_elem_vt(m, t);
    let aux_elem_tys: Vec<_> = aux.iter().map(|&a| table_elem_vt(m, a)).collect();

    // Params: (idx: i32)
    // Returns: (val: elem_ty, aux_vals...)
    let mut returns: Vec<ValType> = vec![elem_ty];
    for aty in &aux_elem_tys {
        returns.push(*aty);
    }
    let ti = m.add_func_type(&[ValType::I32], &returns);

    // Locals: none beyond the idx param (local 0)
    let mut b = FuncBuilder::new(ti);

    // Get the value(s) before nulling out.
    // We need to produce the return values: push them all to the stack first.
    b.emit(Instruction::LocalGet(0)); // idx
    b.emit(Instruction::TableGet(t));
    for &a in aux {
        b.emit(Instruction::LocalGet(0));
        b.emit(Instruction::TableGet(a));
    }

    // Null out the primary table slot.
    b.emit(Instruction::LocalGet(0));
    let null_ref = null_for_elem(elem_ty);
    b.emit(null_ref);
    b.emit(Instruction::TableSet(t));

    // Note: we don't null out aux tables here — the caller is responsible for
    // deciding whether aux entries should be nulled (matching waffle behaviour
    // which also only nulls the primary table).

    b.emit(Instruction::Return);

    let (locals, body) = b.finish();
    Ok(m.add_func(ti, locals, body))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn table_elem_vt(m: &DirectModule, table_idx: u32) -> ValType {
    let imported_tables = m.imports
        .iter()
        .filter(|i| matches!(i.ty, crate::module::DirectImportKind::Table(_)))
        .count() as u32;
    if table_idx < imported_tables {
        // Imported table — look it up.
        let mut ti = 0u32;
        for imp in &m.imports {
            if let crate::module::DirectImportKind::Table(tt) = &imp.ty {
                if ti == table_idx {
                    return ref_ty_to_val(tt.element_type);
                }
                ti += 1;
            }
        }
        panic!("table index out of range");
    } else {
        let local_idx = (table_idx - imported_tables) as usize;
        ref_ty_to_val(m.tables[local_idx].ty.element_type)
    }
}

pub fn ref_ty_to_val(rt: RefType) -> ValType {
    ValType::Ref(rt)
}

fn vt_of_ref(rt: RefType) -> ValType {
    ValType::Ref(rt)
}

fn null_for_elem(vt: ValType) -> Instruction<'static> {
    match vt {
        ValType::Ref(rt) => Instruction::RefNull(rt.heap_type),
        _ => panic!("table element type must be a reference type"),
    }
}
