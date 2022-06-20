mod cli;
pub mod errors;
mod toml;

#[allow(dead_code)]
#[derive(Debug)]
struct Mod {
    id: u32,
    side: ModSide,
}

#[allow(dead_code)]
#[derive(Debug)]
enum ModSide {
    Client,
    Server,
    All,
}

#[allow(dead_code)]
#[derive(Debug)]
enum Loader {
    Fabric,
    Forge,
}

fn main() {
    let res = actual_main();
    if let Err(e) = res {
        println!("{}", e);
    }
}

fn actual_main() -> Result<(), Box<dyn std::error::Error>> {
    let example_pack = std::fs::read_to_string("example_pack.toml")?;
    let pack = toml::parse(example_pack)?;
    println!("{:?}", pack);
    Ok(())
}
