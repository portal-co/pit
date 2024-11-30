use std::collections::BTreeMap;

use anyhow::{bail, Context};
use pit_core::{Arg, Interface, Sig};
use waffle::{
    copying::module::{i2x, x2i, ImportBehavior, ImportFn, State},
    entity::EntityRef,
    Export, ExportKind, ImportKind, Module, Type,
};
use wasmtime_environ::component::*;
use wit_component::DecodedWasm;
pub fn pitify(component: &[u8], module: &mut Module) -> anyhow::Result<()> {
    let decoded = wit_component::decode(component)
        .context("Could not decode component information from bytes.")?;

    let (mut resolve, world_id) = match decoded {
        DecodedWasm::WitPackage(..) => bail!("Cannot instantiate WIT package as module."),
        DecodedWasm::Component(resolve, id) => (resolve, id),
    };

    let adapter_vec = wasmtime_environ::ScopeVec::new();
    let (translation, module_data, component_types) = translate_modules(component, &adapter_vec)?;
    let mut modules = module_data
        .iter()
        .map(|(a, b)| {
            anyhow::Ok((a, {
                let mut m = waffle::Module::from_wasm_bytes(&b.wasm, &Default::default())?;
                m.expand_all_funcs();
                m.without_orig_bytes()
            }))
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
    let core_def = |module: &mut Module, c: CoreDef| {
        anyhow::Ok(match c {
            CoreDef::Export(core_export) => {
                let d = format!(
                    "!{}${}",
                    core_export.instance.as_u32(),
                    match &core_export.item {
                        ExportItem::Index(i) => format!(
                            "i${}",
                            match i {
                                wasmtime_environ::EntityIndex::Function(func_index) =>
                                    format!("f{}", func_index.as_u32()),
                                wasmtime_environ::EntityIndex::Table(table_index) =>
                                    format!("t{}", table_index.as_u32()),
                                wasmtime_environ::EntityIndex::Memory(memory_index) =>
                                    format!("m{}", memory_index.as_u32()),
                                wasmtime_environ::EntityIndex::Global(global_index) =>
                                    format!("g{}", global_index.as_u32()),
                            }
                        ),
                        ExportItem::Name(n) => format!("n${n}"),
                    }
                );
                match module.exports.iter().find_map(|x| {
                    if x.name == d {
                        Some(x.kind.clone())
                    } else {
                        None
                    }
                }) {
                    Some(a) if matches!(a, ExportKind::Func(_) | ExportKind::Global(_)) => {
                        Some(ImportBehavior::Bind(x2i(a)))
                    }
                    _ => Some(ImportBehavior::Passthrough(format!("reflect"), d)),
                }
            }
            CoreDef::InstanceFlags(runtime_component_instance_index) => todo!(),
            CoreDef::Trampoline(trampoline_index) => todo!(),
        })
    };
    for (idx, init) in translation.component.initializers.iter().enumerate() {
        match init {
            GlobalInitializer::InstantiateModule(instantiate_module) => match instantiate_module {
                InstantiateModule::Static(static_module_index, defs) => {
                    let mut m = modules
                        .get(static_module_index)
                        .context("in getting the module")?
                        .imports
                        .iter()
                        .zip(defs.iter())
                        .map(|(a, b)| ((a.module.clone(), a.name.clone()), b.clone()))
                        .collect::<BTreeMap<_, _>>();
                    let mut copier = waffle::copying::module::Copier {
                        src: modules
                            .get_mut(static_module_index)
                            .context("in getting the module")?,
                        dest: &mut *module,
                        state: Box::new(State::new(
                            Box::new(waffle::copying::module::import_fn(|module, a, b| {
                                let v = match m.get(&(a.clone(), b.clone())) {
                                    Some(v) => v.clone(),
                                    None => return Ok(Some(ImportBehavior::Passthrough(a, b))),
                                };

                                core_def(module, v)
                            })),
                            Default::default(),
                            false,
                        )),
                    };
                    for (idx2, mut x) in copier
                        .src
                        .exports
                        .iter()
                        .cloned()
                        .enumerate()
                        .collect::<Vec<_>>()
                    {
                        let old = x.kind.clone();
                        x.kind = i2x(copier.translate_import(x2i(x.kind.clone()))?);
                        x.name = format!("!{idx}$n${}", x.name);
                        copier.dest.exports.push(x.clone());
                        x.name = format!(
                            "!{idx}$i${}",
                            match old {
                                waffle::ExportKind::Table(table) => format!("t{}", table.index()),
                                waffle::ExportKind::Func(func) => format!("f{}", func.index()),
                                waffle::ExportKind::Global(global) =>
                                    format!("g{}", global.index()),
                                waffle::ExportKind::Memory(memory) =>
                                    format!("m{}", memory.index()),
                                    waffle::ExportKind::ControlTag(_) => todo!()
                            }
                        );
                        copier.dest.exports.push(x);
                    }
                }
                InstantiateModule::Import(runtime_import_index, index_map) => todo!(),
            },
            GlobalInitializer::LowerImport { index, import } => todo!(),
            GlobalInitializer::ExtractMemory(extract_memory) => todo!(),
            GlobalInitializer::ExtractRealloc(extract_realloc) => todo!(),
            GlobalInitializer::ExtractPostReturn(extract_post_return) => todo!(),
            GlobalInitializer::Resource(resource) => {
                let wrep = match &resource.rep {
                    wasmtime_environ::WasmValType::I32 => Arg::I32,
                    wasmtime_environ::WasmValType::I64 => Arg::I64,
                    wasmtime_environ::WasmValType::F32 => Arg::F32,
                    wasmtime_environ::WasmValType::F64 => Arg::F64,
                    wasmtime_environ::WasmValType::V128 => todo!(),
                    wasmtime_environ::WasmValType::Ref(wasm_ref_type) => {
                        match wasm_ref_type.heap_type {
                            wasmtime_environ::WasmHeapType::Func => todo!(),
                            wasmtime_environ::WasmHeapType::Extern => Arg::Resource {
                                ty: pit_core::ResTy::None,
                                nullable: wasm_ref_type.nullable,
                                take: false,
                                ann: vec![],
                            },
                            wasmtime_environ::WasmHeapType::TypedFunc(
                                module_interned_type_index,
                            ) => todo!(),
                        }
                    }
                };
                let resid = format!("component-r{}", idx);
                let prs = Interface {
                    methods: std::iter::once((
                        resid.clone(),
                        Sig {
                            params: vec![],
                            rets: vec![wrep],
                            ann: vec![],
                        },
                    ))
                    .collect(),
                    ann: vec![],
                };
                let destruct = match &resource.dtor {
                    None => None,
                    Some(d) => {
                        let Some(ImportBehavior::Bind(ImportKind::Func(f))) =
                            core_def(module, d.clone())?
                        else {
                            anyhow::bail!("invalid typed or recursive core def")
                        };
                        Some(f)
                    }
                };
                let c = pit_patch::util::canon(module, &prs, destruct, &resid);
                module.exports.push(Export {
                    name: resid,
                    kind: ExportKind::Func(c),
                });
            }
        }
    }

    Ok(())
}

fn translate_modules<'a>(
    bytes: &'a [u8],
    scope: &'a wasmtime_environ::ScopeVec<u8>,
) -> anyhow::Result<(
    ComponentTranslation,
    wasmtime_environ::PrimaryMap<StaticModuleIndex, wasmtime_environ::ModuleTranslation<'a>>,
    wasmtime_environ::component::ComponentTypes,
)> {
    let tunables = wasmtime_environ::Tunables::default_u32();
    let mut types = ComponentTypesBuilder::default();
    let mut validator = wasmtime_environ::wasmparser::Validator::new_with_features(
        wasmtime_environ::wasmparser::WasmFeatures::all(),
    );

    let (translation, modules) = Translator::new(&tunables, &mut validator, &mut types, scope)
        .translate(bytes)
        .context("Could not translate input component to core WASM.")?;

    Ok((
        translation,
        modules,
        types.finish(&Default::default(), [], []).0,
    ))
}
