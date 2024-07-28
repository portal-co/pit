use anyhow::{anyhow, Context};
use pit_rust_guest::Opts;
use quote::quote;
fn main() -> anyhow::Result<()>{
    let mut a = std::env::args();
    a.next();
    let src = a.next().context("in getting the src")?;
    let src = std::fs::read_to_string(src)?;
    let (_,src) = pit_core::parse_interface(&src).map_err(|e|anyhow!("invalid interface: {e}"))?;
    let mut x = Opts{
        root: quote!{
            
        },
        salt: std::env::var("PIT_SALT").unwrap_or(format!("")).into_bytes(),
    };

    let v = pit_rust_guest::render(&x, &src);
    let v = prettyplease::unparse(&syn::parse_file(&v.to_string())?);
    let Some(dst) = a.next() else{
        println!("{v}");
        return Ok(());
    };
    std::fs::write(dst, v.to_string())?;
    Ok(())
}
