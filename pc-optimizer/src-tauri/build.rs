fn main() {
    // O `tauri_build` embute o ícone no executável, mas não declara os arquivos
    // de ícone como dependência. Sem estas linhas, trocar a logo não reconstrói
    // nada e o programa continua saindo com o ícone antigo.
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=icons/icon.ico");

    // O manifesto vai pelo linker, para TODOS os alvos, e não pelo tauri-build, que só o põe no executável. Os testes
    // que tocam no Tauri (o contrato da interface, `src/contrato.rs`) importam o comctl32 6, e sem o manifesto o
    // binário de teste não abre. O conteúdo é o mesmo que o tauri-build punha: o cliente não vê diferença.
    let mut atributos = tauri_build::Attributes::new();
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let manifesto = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows")
            .join("app.manifest");
        println!("cargo:rerun-if-changed=windows/app.manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifesto.display());
        atributos = atributos.windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    }

    tauri_build::try_build(atributos).expect("tauri-build falhou")
}
