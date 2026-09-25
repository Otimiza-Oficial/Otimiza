// Trava de cobertura da esteira: módulo com teste que nenhum passo do release.yml executa vira teste vermelho.
// Oito módulos já ficaram fora sem nada acusar, com o painel todo verde.

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    const WORKFLOW: &str = "../../.github/workflows/release.yml";

    fn module_path(src_root: &Path, file: &Path) -> Option<String> {
        let relative = file.strip_prefix(src_root).ok()?;
        let mut partes: Vec<String> = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();

        let ultimo = partes.pop()?;
        let nome = ultimo.strip_suffix(".rs")?;

        if nome != "mod" && nome != "lib" && nome != "main" {
            partes.push(nome.to_string());
        }

        (!partes.is_empty()).then(|| partes.join("::"))
    }

    /// Pela estrutura (`#[cfg(test)]` seguido de `mod`, com qualquer nome): procurar o texto `mod tests` deixava
    /// invisível um módulo de teste com outro nome.
    fn tem_modulo_de_teste(conteudo: &str) -> bool {
        conteudo.split("#[cfg(test)]").skip(1).any(|depois| {
            depois
                .lines()
                .map(str::trim_start)
                .find(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with("#["))
                .map(|l| l.starts_with("mod ") || l.starts_with("pub mod "))
                .unwrap_or(false)
        })
    }

    fn modulos_com_teste(dir: &Path, src_root: &Path, achados: &mut BTreeSet<String>) {
        let Ok(entradas) = std::fs::read_dir(dir) else {
            return;
        };

        for entrada in entradas.flatten() {
            let caminho = entrada.path();

            if caminho.is_dir() {
                modulos_com_teste(&caminho, src_root, achados);
                continue;
            }

            if caminho.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }

            let Ok(conteudo) = std::fs::read_to_string(&caminho) else {
                continue;
            };

            // Esta trava não é módulo do produto; a esteira a roda junto com o núcleo.
            if caminho.file_name().and_then(|n| n.to_str()) == Some("ci_coverage.rs") {
                continue;
            }

            if tem_modulo_de_teste(&conteudo) {
                if let Some(caminho_modulo) = module_path(src_root, &caminho) {
                    achados.insert(caminho_modulo);
                }
            }
        }
    }

    #[test]
    fn todo_modulo_com_teste_roda_na_esteira() {
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let src = raiz.join("src");

        let yaml = std::fs::read_to_string(raiz.join(WORKFLOW))
            .expect("o arquivo da esteira precisa existir para esta trava fazer sentido");

        let mut modulos = BTreeSet::new();
        modulos_com_teste(&src, &src, &mut modulos);

        assert!(
            modulos.len() > 15,
            "a varredura achou só {} módulos com teste — provavelmente o caminho está errado",
            modulos.len()
        );

        // Como palavras inteiras: toda linha tem `modules::`, e buscar texto solto aprovaria qualquer módulo.
        let filtros: BTreeSet<&str> = yaml
            .split_whitespace()
            .map(|token| token.trim_matches(['"', '\'']))
            .filter(|token| token.contains("::"))
            .collect();

        assert!(
            !filtros.is_empty(),
            "nenhum filtro de teste encontrado na esteira — o formato do YAML mudou"
        );

        // A comparação vai nos dois sentidos: um passo pode mirar o módulo inteiro ou só o submódulo de teste.
        let esquecidos: Vec<&String> = modulos
            .iter()
            .filter(|modulo| {
                let namespace = format!("{}::tests", modulo);

                !filtros.iter().any(|filtro| {
                    let alvo = filtro.trim_end_matches(':');
                    namespace.starts_with(alvo) || alvo.starts_with(&namespace)
                })
            })
            .collect();

        assert!(
            esquecidos.is_empty(),
            "estes módulos têm teste mas nenhum passo da esteira os executa: {:?}\n\
             Acrescente-os a um passo em .github/workflows/release.yml — sem isso a \
             versão é publicada com o painel verde e os testes nunca rodados.",
            esquecidos
        );
    }

    #[test]
    fn caminho_de_modulo_sai_do_caminho_de_arquivo() {
        let src = Path::new("src");

        assert_eq!(
            module_path(src, Path::new("src/modules/windows/health.rs")).as_deref(),
            Some("modules::windows::health")
        );
        assert_eq!(
            module_path(src, Path::new("src/modules/windows/mod.rs")).as_deref(),
            Some("modules::windows")
        );
        assert_eq!(
            module_path(src, Path::new("src/core/platform.rs")).as_deref(),
            Some("core::platform")
        );
        assert_eq!(module_path(src, Path::new("src/lib.rs")), None);
    }

    /// A versão do binário precisa bater com a dos três arquivos que a declaram: a 1.5 quase saiu dizendo 1.3.0,
    /// e a faixa de atualização mandaria o cliente baixar a versão que acabou de instalar.
    #[test]
    fn as_quatro_declaracoes_de_versao_concordam() {
        let esperada = env!("CARGO_PKG_VERSION");

        // `unwrap` de propósito: arquivo sumido ou sem a chave é quebra de estrutura, e precisa parar a esteira.
        let ler = |caminho: &str, chave_ate: usize| -> String {
            let bruto = std::fs::read_to_string(caminho)
                .unwrap_or_else(|e| panic!("não consegui ler {}: {}", caminho, e));
            let v: serde_json::Value = serde_json::from_str(&bruto)
                .unwrap_or_else(|e| panic!("{} não é JSON válido: {}", caminho, e));

            // A versão do pacote raiz do lock fica em `packages[""].version`.
            let achado = if chave_ate == 1 {
                v.get("version").cloned()
            } else {
                v.pointer("/packages//version").cloned()
            };

            achado
                .and_then(|x| x.as_str().map(str::to_string))
                .unwrap_or_else(|| panic!("{} não declara versão", caminho))
        };

        let conf = ler("tauri.conf.json", 1);
        let pkg = ler("../package.json", 1);
        let lock = ler("../package-lock.json", 1);
        let lock_raiz = ler("../package-lock.json", 2);

        assert_eq!(
            conf, esperada,
            "tauri.conf.json diz {} e o Cargo.toml diz {} — o instalador sairia com um              número e o binário com outro",
            conf, esperada
        );
        assert_eq!(pkg, esperada, "package.json diz {} e o Cargo.toml diz {}", pkg, esperada);
        assert_eq!(
            lock, esperada,
            "package-lock.json diz {} e o Cargo.toml diz {}",
            lock, esperada
        );
        assert_eq!(
            lock_raiz, esperada,
            "package-lock.json, em packages[\"\"], diz {} e o Cargo.toml diz {}",
            lock_raiz, esperada
        );
    }
}
