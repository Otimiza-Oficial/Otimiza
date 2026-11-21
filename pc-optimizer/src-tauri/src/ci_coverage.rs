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
            "tauri.conf.json diz {} e o Cargo.toml diz {} — o instalador sairia com um número e o binário com outro",
            conf, esperada
        );
        assert_eq!(
            pkg, esperada,
            "package.json diz {} e o Cargo.toml diz {}",
            pkg, esperada
        );
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

    /// **Pura.** Um texto com um buraco de 8 ou mais espaços no meio, dentro de aspas: é a barra de continuação de linha
    /// (`\` no fim da linha) que se perdeu. Desde 29/08 a mensagem "ative com a sua chave" saía para o cliente com
    /// "ainda não          está ativado", e outros quinze textos do mesmo jeito.
    fn buraco_em_texto(linha: &str) -> bool {
        let conteudo = linha.trim_start();
        if conteudo.starts_with("//") {
            return false;
        }
        let deslocamento = linha.len() - conteudo.len();
        let bytes = linha.as_bytes();
        let (mut dentro, mut i) = (false, deslocamento);
        while i < bytes.len() {
            match bytes[i] {
                b'\\' => i += 2,
                b'"' => {
                    dentro = !dentro;
                    i += 1;
                }
                b' ' if dentro => {
                    let fim = (i..bytes.len()).find(|&j| bytes[j] != b' ').unwrap_or(bytes.len());
                    let antes = bytes[i - 1];
                    if fim - i >= 8 && antes != b'"' && fim < bytes.len() && bytes[fim] != b'"' {
                        return true;
                    }
                    i = fim;
                }
                _ => i += 1,
            }
        }
        false
    }

    #[test]
    fn buraco_em_texto_acha_a_continuacao_perdida_e_ignora_indentacao() {
        assert!(buraco_em_texto(r#"    Err("ainda não          está ativado")"#));
        assert!(!buraco_em_texto(r#"    Err("ainda não \"#), "a continuação certa");
        assert!(!buraco_em_texto(r#"             está ativado.")"#), "indentação da linha seguinte");
        assert!(!buraco_em_texto(r#"    // texto          em comentário"#));
        assert!(!buraco_em_texto(r#"    let x = 1;          // alinhado"#));
    }

    /// Textos que alinham colunas de propósito (saídas de comando do Windows nos testes, o relatório do sistema).
    const ALINHADOS_DE_PROPOSITO: &[&str] = &[
        "cabecalho.rs", "firmware.rs", "power.rs", "cs2.rs", "readiness.rs", "crashes.rs", "pacote.rs", "ci_coverage.rs",
    ];

    #[test]
    fn nenhum_texto_tem_a_continuacao_de_linha_perdida() {
        fn varrer(dir: &Path, achados: &mut Vec<String>) {
            let Ok(entradas) = std::fs::read_dir(dir) else { return };
            for entrada in entradas.flatten() {
                let caminho = entrada.path();
                if caminho.is_dir() {
                    varrer(&caminho, achados);
                    continue;
                }
                let nome = caminho.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
                if !nome.ends_with(".rs") || ALINHADOS_DE_PROPOSITO.contains(&nome.as_str()) {
                    continue;
                }
                let Ok(conteudo) = std::fs::read_to_string(&caminho) else { continue };
                for (n, linha) in conteudo.lines().enumerate() {
                    if buraco_em_texto(linha) {
                        achados.push(format!("{}:{}", caminho.display(), n + 1));
                    }
                }
            }
        }
        let mut achados = Vec::new();
        varrer(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").as_path(), &mut achados);
        assert!(
            achados.is_empty(),
            "texto com a barra de continuação de linha perdida (um buraco de espaços no meio da frase): {:?}",
            achados
        );
    }

    /// **Pura.** O EULA em RTF, a partir do texto de `EULA.txt`. RTF e não o texto: o NSIS e o MSI aceitam RTF como
    /// está, e o MSI converte texto puro para RTF em página de código 1252, o que estragaria cada acento. Só ASCII
    /// sai daqui: o que não é ASCII vira `\uN?` (UTF-16 com sinal, o que o RTF pede).
    fn eula_em_rtf(texto: &str) -> String {
        let mut rtf = String::from(
            "{\\rtf1\\ansi\\ansicpg1252\\deff0\\uc1{\\fonttbl{\\f0\\fswiss\\fcharset0 Segoe UI;}}\\f0\\fs18\n",
        );
        for linha in texto.trim_start_matches('\u{feff}').lines() {
            for c in linha.chars() {
                match c {
                    '\\' | '{' | '}' => {
                        rtf.push('\\');
                        rtf.push(c);
                    }
                    ' '..='~' => rtf.push(c),
                    _ => {
                        let mut unidades = [0u16; 2];
                        for u in c.encode_utf16(&mut unidades) {
                            rtf.push_str(&format!("\\u{}?", *u as i16));
                        }
                    }
                }
            }
            rtf.push_str("\\par\n");
        }
        rtf.push_str("}\n");
        rtf
    }

    #[test]
    fn eula_em_rtf_escapa_acento_e_chave() {
        assert!(eula_em_rtf("é {a}\\").contains("\\u233? \\{a\\}\\\\\\par"));
        assert!(eula_em_rtf("\u{feff}Licença").contains("Licen\\u231?a\\par"), "o BOM não entra no texto");
        assert!(eula_em_rtf("😀").contains("\\u-10179?\\u-8704?"), "fora do BMP vira par de UTF-16");
    }

    /// 1.0.12 (8.1): o instalador mostra o EULA, e o EULA que ele mostra é o de `EULA.txt`. Mudou o texto, regenere:
    /// `OTIMIZA_GERAR_EULA=1 cargo test --lib ci_coverage::tests::o_instalador_mostra_o_eula_do_texto`.
    #[test]
    fn o_instalador_mostra_o_eula_do_texto() {
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let conf: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(raiz.join("tauri.conf.json")).expect("tauri.conf.json"),
        )
        .expect("tauri.conf.json é JSON");
        assert_eq!(
            conf.pointer("/bundle/licenseFile").and_then(|v| v.as_str()),
            Some("EULA.rtf"),
            "o instalador precisa mostrar o EULA (bundle.licenseFile)"
        );
        assert!(
            conf.pointer("/bundle/resources/..~1LICENCAS-DE-TERCEIROS.md").is_some(),
            "os avisos de terceiros (FSR, PresentMon) precisam ir junto com o programa"
        );

        let texto = std::fs::read_to_string(raiz.join("EULA.txt")).expect("EULA.txt");
        let esperado = eula_em_rtf(&texto);
        if std::env::var_os("OTIMIZA_GERAR_EULA").is_some() {
            std::fs::write(raiz.join("EULA.rtf"), &esperado).expect("gravar EULA.rtf");
        }
        // O checkout no Windows troca LF por CRLF (`* text=auto`); a comparação é do conteúdo.
        let gravado = std::fs::read_to_string(raiz.join("EULA.rtf"))
            .expect("EULA.rtf (gere com OTIMIZA_GERAR_EULA=1)")
            .replace("\r\n", "\n");
        assert!(gravado.is_ascii(), "EULA.rtf com byte fora do ASCII: o acento sairia estragado");
        assert!(
            gravado == esperado,
            "EULA.rtf não é o texto de EULA.txt: regenere com OTIMIZA_GERAR_EULA=1"
        );
    }
}
