use std::{collections::BTreeSet, fs, iter::once};

use anyhow::Context;
use base64::Engine;
use pit_patch::{get_interfaces, lower::Cfg};
use waffle::{
    copying::module::{i2x, import_fn, tree_shake, x2i, Copier, ImportBehavior, State},
    ImportKind,
};

const TEAVM_INTEROP_VER: &'static str = "0.10.2";

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();
    args.next();
    match args.next().context("in getting the subcommand")?.as_str() {
        "untpit" => {
            let a = args.next().context("in getting the input")?;
            let a = std::fs::read(a)?;
            let mut m = waffle::Module::from_wasm_bytes(&a, &Default::default())?;
            pit_patch::tpit::wrap(&mut m)?;
            let b = args.next().context("in getting the output")?;
            std::fs::write(b, m.to_wasm_bytes()?)?;
        }
        "jigger" => {
            let a = args.next().context("in getting the input")?;
            let a = std::fs::read(a)?;
            let mut m = waffle::Module::from_wasm_bytes(&a, &Default::default())?;
            pit_patch::canon::jigger(&mut m, &[])?;
            let b = loop {
                let b = args.next().context("in getting the output")?;
                let Some(c) = b.strip_prefix("-") else {
                    break b;
                };
                pit_patch::canon::jigger(&mut m, c.as_bytes())?;
            };
            std::fs::write(b, m.to_wasm_bytes()?)?;
        }
        "lower" => {
            let a = args.next().context("in getting the input")?;
            let a = std::fs::read(a)?;
            let mut m = waffle::Module::from_wasm_bytes(&a, &Default::default())?;
            m.expand_all_funcs()?;
            let b = loop {
                let b = args.next().context("in getting the output")?;
                let Some(c) = b.strip_prefix("-") else {
                    break b;
                };
            };
            pit_patch::lower::instantiate(
                &mut m,
                &Cfg {
                    unexportable_i32_tables: false,
                },
            )?;
            tree_shake(&mut m)?;
            std::fs::write(b, m.to_wasm_bytes()?)?;
        }
        "embed" => {
            let a = args.next().context("in getting the input")?;
            let a = std::fs::read(a)?;
            let mut m = waffle::Module::from_wasm_bytes(&a, &Default::default())?;
            m.expand_all_funcs()?;
            let b = loop {
                let b = args.next().context("in getting the output")?;
                let Some(c) = b.strip_prefix("-") else {
                    break b;
                };
                let c = std::fs::read_to_string(c)?;
                let Ok((_, c)) = pit_core::parse_interface(&c) else {
                    anyhow::bail!("invalid interface");
                };
                m.custom_sections
                    .entry(format!(".pit-types"))
                    .or_default()
                    .extend(c.to_string().as_bytes().iter().cloned().chain(once(0)));
            };
            let c = match get_interfaces(&m) {
                Ok(a) => a,
                _ => vec![],
            };
            let c = c.into_iter().collect::<BTreeSet<_>>();
            *m.custom_sections.entry(format!(".pit-types")).or_default() = c
                .iter()
                .flat_map(|a| {
                    a.to_string()
                        .bytes()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .chain(once(0))
                })
                .collect();
            std::fs::write(b, m.to_wasm_bytes()?)?;
        }
        "rust-guest" => {
            let a = args.next().context("in getting the input")?;
            let a = std::fs::read_to_string(a)?;
            let Ok((_, a)) = pit_core::parse_interface(&a) else {
                anyhow::bail!("invalid interface");
            };
            let mut opts = pit_rust_guest::Opts {
                root: quote::quote! {::tpit_rt},
                salt: vec![],
                tpit: true,
            };
            let b = loop {
                let b = args.next().context("in getting the output")?;
                let Some(c) = b.strip_prefix("-") else {
                    break b;
                };
                if c == "extern-externref" {
                    opts.tpit = false;
                }
                if c == "salt" {
                    opts.salt.extend(
                        args.next()
                            .iter()
                            .flat_map(|a| a.as_bytes().iter())
                            .cloned(),
                    );
                }
                if c == "root" {
                    opts.root = syn::parse_str(&args.next().context("in getting the root")?)?;
                }
            };
            let a = pit_rust_guest::render(&opts, &a);
            let a = syn::parse2(a)?;
            std::fs::write(b, prettyplease::unparse(&a))?;
        }
        "teavm" => {
            let a = args.next().context("in getting the input")?;
            let a = std::fs::read_to_string(a)?;
            let Ok((_, a)) = pit_core::parse_interface(&a) else {
                anyhow::bail!("invalid interface");
            };
            let mut pkg = format!("pc.portal.pit.guest");
            let mut binders = pit_teavm::Binders::default();
            let b = loop {
                let b = args.next().context("in getting the output")?;
                let Some(c) = b.strip_prefix("-") else {
                    break b;
                };
                if c == "pkg" {
                    pkg = args.next().context("in getting the package")?;
                }
            };
            let a = pit_teavm::emit(&a, &pkg, &binders);
            // let a = syn::parse2(a)?;
            std::fs::write(b, a)?;
        }
        "gen-c" => {
            let a = args.next().context("in getting the input")?;
            let a = std::fs::read_to_string(a)?;
            let Ok((_, a)) = pit_core::parse_interface(&a) else {
                anyhow::bail!("invalid interface");
            };
            // let mut pkg = format!("pc.portal.pit.guest");
            // let mut binders = pit_teavm::Binders::default();
            let b = loop {
                let b = args.next().context("in getting the output")?;
                let Some(c) = b.strip_prefix("-") else {
                    break b;
                };
                // if c == "pkg" {
                //     pkg = args.next().context("in getting the package")?;
                // }
            };
            let a = pit_c::cify(&a);
            // let a = syn::parse2(a)?;
            std::fs::write(b, a)?;
        }
        "package" => {
            let a = args.next().context("in getting the input")?;
            let a = std::fs::read_to_string(a)?;
            let Ok((_, a)) = pit_core::parse_interface(&a) else {
                anyhow::bail!("invalid interface");
            };
            // let mut pkg = format!("pc.portal.pit.guest");
            // let mut binders = pit_teavm::Binders::default();            
            let mut pkg = format!("pc.portal.pit.guest");
            let mut binders = pit_teavm::Binders::default();
            let mut opts = pit_rust_guest::Opts {
                root: quote::quote! {::tpit_rt},
                salt: vec![],
                tpit: true,
            };
            let b = loop {
                let b = args.next().context("in getting the output")?;
                let Some(c) = b.strip_prefix("-") else {
                    break b;
                };
                if c == "scala/pkg" {
                    pkg = args.next().context("in getting the package")?;
                };
                if c == "rust/extern-externref" {
                    opts.tpit = false;
                }
                if c == "rust/salt" {
                    opts.salt.extend(
                        args.next()
                            .iter()
                            .flat_map(|a| a.as_bytes().iter())
                            .cloned(),
                    );
                }
                if c == "rust/root" {
                    opts.root = syn::parse_str(&args.next().context("in getting the root")?)?;
                }
            };
            let rid = a.rid_str();
            let a2 = pit_rust_guest::render(&opts, &a);
            let a2 = syn::parse2(a2)?;
            std::fs::write(format!("{b}/src/lib.rs"), prettyplease::unparse(&a2))?;
            let a2 = pit_teavm::emit(&a, &pkg, &binders);
            let scala_path = format!("{}/R{}.scala",a.rid_str(),pkg.replace(".", "/"));
            std::fs::write(format!("{b}/{}",scala_path), a2)?;
            std::fs::write(format!("{b}/R{}.h",a.rid_str()), pit_c::cify(&a))?;
            std::fs::write(format!("{b}/R{rid}.c"), format!(r#"
            #define R{rid}_IMPL
            #include <R{rid}.h>
            "#))?;
            let bazel_cc = format!(r#"
            cc_library(
                name = "r{rid}",
                srcs = ["R{rid}.c"],
                hdrs = ["R{rid}.h"],
                visibility = ["//visibility:public"],
                deps = ["@wasm_handler"]
            )
            "#);
            let crate_name = format!("pit-autogen-{}-{}",base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(a.rid()),if opts.tpit{
                "tpit"
            }else{
                "externref"
            });
            std::fs::write(format!("{b}/Cargo.toml"), format!(r#"
            [package]
            name = "{crate_name}"
            version = "{}"
            edition = "2021"
            license = "CC0-1.0"
            description = "Automatically generated"

            [dependencies]
            {}

            [package.metadata.bazel]
            additive_build_file_content = """
            {bazel_cc}
            """
            extra_aliased_targets = {{ r{rid} = "r{rid}" }}
            "#,env!("CARGO_PKG_VERSION"),if opts.tpit{
                format!(r#"tpit-rt = {{ version = "{}"}}"#,env!("CARGO_PKG_VERSION"))
            }else{
                format!(r#"externref = "0.2.0""#)
            }))?;
            std::fs::write(format!("{b}/BUILD.bazel"), format!(r#"
            package(default_visibility = ["//visibility:public"])

            load("@rules_rust//rust:defs.bzl", "rust_library")
            load("@io_bazel_rules_scala//scala:scala.bzl", "scala_library", "scala_binary", "scala_test")
            {bazel_cc}
            rust_library(
                name = "{crate_name}",
                srcs = [
                    "src/lib.rs",
                ],
                deps = ["@crates//:{}"]
            )
            scala_library(
                name = "teavm-{rid}",
                srcs = [
                    "{scala_path}",
                ],
                deps = [
                    "@org-teavm-teavm-interop"
                ],
            )
            "#,if opts.tpit{
                "tpit-rt"
            }else{
                "externref"
            }))?;
            std::fs::write(format!("{b}/build.sbt"), format!(r#"
            // https://mvnrepository.com/artifact/org.teavm/teavm-interop
            ThisBuild / scalaVersion := "3.3.3"
            libraryDependencies += "org.teavm" % "teavm-interop" % "{TEAVM_INTEROP_VER}"

            "#))?;
            std::fs::write(format!("{b}/CMakeLists.txt"), format!(r#"
            add_library(r{rid} STATIC R{rid}.c)
            target_include_directories(r{rid} PUBLIC ${{CMAKE_CURRENT_SOURCE_DIR}})
            target_link_libraries(r{rid} PUBLIC wasm_handler)
            "#))?;
        }
        "hash" => {
            loop{
                let ap = args.next().context("in getting the input")?;
                let a = std::fs::read_to_string(&ap)?;
                let Ok((_, a)) = pit_core::parse_interface(&a) else {
                    anyhow::bail!("invalid interface");
                };
                println!("{ap}: {}",a.rid_str());
            }
        }
        _ => anyhow::bail!(
            "invalid command (valid ones are: untpit, jigger, lower, rust-guest, teavm, embed, hash, gen-c)"
        ),
    };
    Ok(())
}
