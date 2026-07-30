fn main() {
    // O `tauri_build` embute o ícone no executável, mas não declara os arquivos
    // de ícone como dependência. Sem estas linhas, trocar a logo não reconstrói
    // nada e o programa continua saindo com o ícone antigo.
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=icons/icon.ico");

    tauri_build::build()
}
