use anyhow::Result;

pub fn postprocess_wasm(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut config = walrus::ModuleConfig::new();
    config
        .generate_name_section(false)
        .generate_producers_section(false)
        .generate_dwarf(false);

    let mut module = config.parse(bytes)?;
    walrus::passes::gc::run(&mut module);

    let custom_sections = module.customs.iter().map(|(id, _)| id).collect::<Vec<_>>();
    for id in custom_sections {
        let _ = module.customs.delete(id);
    }

    Ok(module.emit_wasm())
}
