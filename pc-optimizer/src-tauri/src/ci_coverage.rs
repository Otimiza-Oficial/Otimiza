// Trava de cobertura da esteira
//
// Os testes rodam no CI separados por área, um passo por assunto, porque o log
// detalhado só é acessível com autenticação e a divisão em passos faz o próprio
// painel apontar onde quebrou.
//
// O preço dessa escolha é que a lista é escrita à mão: quem cria um módulo novo
// precisa lembrar de acrescentá-lo ao YAML. Já esqueceu — oito módulos ficaram
// fora da esteira sem que nada acusasse, e a versão foi publicada com o painel
// todo verde mostrando testes que nunca rodaram.
//
// Este arquivo transforma esse esquecimento em teste vermelho.

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    const WORKFLOW: &str = "../../.github/workflows/release.yml";

    /// Converte o caminho de um arquivo-fonte no caminho de módulo do Rust.
    ///
    /// `src/modules/windows/health.rs` vira `modules::windows::health`, e
    /// `src/modules/windows/mod.rs` vira `modules::windows`.
    fn module_path(src_root: &Path, file: &Path) -> Option<String> {
        let relative = file.strip_prefix(src_root).ok()?;
        let mut partes: Vec<String> = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();

        let ultimo = partes.pop()?;
        let nome = ultimo.strip_suffix(".rs")?;

        // `mod.rs` não acrescenta nível: o módulo é a própria pasta.
        if nome != "mod" && nome != "lib" && nome != "main" {
            partes.push(nome.to_string());
        }

        (!partes.is_empty()).then(|| partes.join("::"))
    }

    /// Todos os arquivos-fonte que declaram `mod tests`.
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

            // O próprio arquivo desta trava fica de fora: ele não é um módulo do
            // produto, e a esteira roda a trava junto com o núcleo.
            if caminho.file_name().and_then(|n| n.to_str()) == Some("ci_coverage.rs") {
                continue;
            }

            if conteudo.contains("mod tests") {
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

        // Os filtros de verdade, extraídos do YAML como palavras inteiras.
        //
        // Buscar o texto solto não serve: `modules::windows::health` contém
        // `modules::`, e como toda linha do arquivo tem `modules::`, um teste
        // escrito assim aprovaria qualquer módulo — inclusive os que ninguém
        // executa. Foi exatamente esse o erro que esta trava existe para pegar.
        let filtros: BTreeSet<&str> = yaml
            .split_whitespace()
            .map(|token| token.trim_matches(['"', '\'']))
            .filter(|token| token.contains("::"))
            .collect();

        assert!(
            !filtros.is_empty(),
            "nenhum filtro de teste encontrado na esteira — o formato do YAML mudou"
        );

        // Os testes de um módulo vivem sob `<módulo>::tests`, e é esse nome que
        // o filtro do cargo compara. A comparação vai nos dois sentidos porque
        // as duas formas aparecem na esteira: um passo pode mirar o módulo
        // inteiro (`modules::windows::health::`) ou só o submódulo de teste
        // dele (`modules::windows::tests::`, o motor de otimização).
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
        // `mod.rs` é a pasta, não um nível a mais.
        assert_eq!(
            module_path(src, Path::new("src/modules/windows/mod.rs")).as_deref(),
            Some("modules::windows")
        );
        assert_eq!(
            module_path(src, Path::new("src/core/platform.rs")).as_deref(),
            Some("core::platform")
        );
        // A raiz da biblioteca não vira caminho de módulo.
        assert_eq!(module_path(src, Path::new("src/lib.rs")), None);
    }
}
