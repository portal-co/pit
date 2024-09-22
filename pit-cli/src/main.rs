use anyhow::Context;

fn main() -> anyhow::Result<()>{
    let mut args = std::env::args();
    args.next();
    match args.next().context("in getting the subcommand")?.as_str(){
        "untpit" => {
            let a = args.next().context("in getting the input")?;
            let a = std::fs::read(a)?;
            let mut m = waffle::Module::from_wasm_bytes(&a, &Default::default())?;
            pit_patch::tpit::wrap(&mut m)?;
            let b = args.next().context("in getting the output")?;
            std::fs::write(b, m.to_wasm_bytes()?)?;
        },
        "jigger" => {
            let a = args.next().context("in getting the input")?;
            let a = std::fs::read(a)?;
            let mut m = waffle::Module::from_wasm_bytes(&a, &Default::default())?;
            pit_patch::canon::jigger(&mut m)?;
            let b = args.next().context("in getting the output")?;
            std::fs::write(b, m.to_wasm_bytes()?)?;
        },
        _ => anyhow::bail!("invalid command (valis ones are: untpit, jigger)")
    };
    Ok(())
}
