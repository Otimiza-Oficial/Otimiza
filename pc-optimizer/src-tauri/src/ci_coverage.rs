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

    /// O arquivo declara algum módulo de teste?
    ///
    /// Procurar pelo texto `mod tests` não bastava: um módulo de teste com
    /// outro nome — `mod classificacao`, por exemplo — ficava invisível para
    /// esta trava, e os testes dele nunca entrariam na esteira. É o mesmo
    /// esquecimento que este arquivo existe para pegar, só que uma camada
    /// acima.
    ///
    /// O que vale é a estrutura: um `#[cfg(test)]` seguido de uma declaração
    /// de módulo, com qualquer nome.
    fn tem_modulo_de_teste(conteudo: &str) -> bool {
        conteudo.split("#[cfg(test)]").skip(1).any(|depois| {
            depois
                .lines()
                .map(str::trim_start)
                // Entre o `#[cfg(test)]` e o `mod` cabem outros atributos e
                // comentários — `#[path = "..."]`, por exemplo.
                .find(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with("#["))
                .map(|l| l.starts_with("mod ") || l.starts_with("pub mod "))
                .unwrap_or(false)
        })
    }

    /// Todos os arquivos-fonte que declaram um módulo de teste.
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

    /// A versão do binário precisa bater com a dos três arquivos que a
    /// declaram.
    ///
    /// Por que isto virou teste: a 1.5 chegou pronta para publicar com os
    /// quatro arquivos ainda dizendo `1.3.0`. A esteira não teria acusado —
    /// ela só confere que as notas de versão CITAM o número, não que o
    /// binário o carrega. O estrago seria visível na primeira abertura: a
    /// faixa de atualização compara `CARGO_PKG_VERSION` com a última
    /// publicada e mandaria o cliente baixar a versão que ele acabou de
    /// instalar, num laço que o "fechei" não interrompe. E o relatório que
    /// o cliente cola no atendimento anunciaria a versão errada, mandando
    /// o suporte investigar bug em código que não está rodando.
    #[test]
    fn as_quatro_declaracoes_de_versao_concordam() {
        let esperada = env!("CARGO_PKG_VERSION");

        // `unwrap` de propósito: se um destes arquivos sumiu ou deixou de
        // ter a chave, isto é uma quebra de estrutura do projeto e precisa
        // parar a esteira em vez de passar batido.
        let ler = |caminho: &str, chave_ate: usize| -> String {
            let bruto = std::fs::read_to_string(caminho)
                .unwrap_or_else(|e| panic!("não consegui ler {}: {}", caminho, e));
            let v: serde_json::Value = serde_json::from_str(&bruto)
                .unwrap_or_else(|e| panic!("{} não é JSON válido: {}", caminho, e));

            // `chave_ate` distingue a versão do pacote raiz do lock (que fica
            // em `packages[""].version`) da versão do arquivo.
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
