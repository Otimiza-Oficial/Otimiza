// O navegador
//
// Em PC fraco, o navegador costuma ser o programa que mais consome memória —
// mais que todo o resto junto. Nenhum otimizador do mercado olha para dentro
// dele, e é onde está boa parte da dor.
//
// O QUE A INVESTIGAÇÃO MUDOU NESTE DESENHO
//
// A hipótese inicial era "24 extensões consumindo 1,8 GB". Ela não se sustenta:
// medindo de fora do navegador, NÃO EXISTE jeito de saber quanto cada extensão
// gasta de memória. Várias dividem um mesmo processo, e a linha de comando dele
// não diz quais. O Gerenciador de Tarefas do próprio Chrome consegue porque roda
// dentro do processo; nós não. Qualquer número "por extensão" que este programa
// mostrasse seria inventado — então ele não mostra.
//
// Na máquina onde isto foi investigado o agregado das extensões era 39 MB, e as
// ABAS custavam 665 MB, com uma única aba em 709 MB. O número honesto e que o
// cliente sente ao fechar é o do navegador inteiro.
//
// A LINHA QUE NÃO SE ATRAVESSA
//
// Este módulo lê manifesto de extensão, traduções de extensão e TAMANHO de
// pastas. Nunca abre `History`, `Cookies`, `Login Data`, `Web Data`,
// `Bookmarks`, `Top Sites` ou `Sessions`. Nem para contar. E ao medir o
// `IndexedDB` soma só o total: os nomes das subpastas ali dentro revelam quais
// sites a pessoa usa.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Pastas cujo conteúdo é descartável: o navegador refaz sozinho.
///
/// Lista fechada e conservadora. Cada uma foi conferida individualmente — o
/// critério não é "parece cache pelo nome".
const CACHE_DESCARTAVEL: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "DawnGraphiteCache",
    "DawnWebGPUCache",
    "Shared Dictionary",
];

/// Pastas que PARECEM cache e não são.
///
/// `IndexedDB` guarda dado de aplicativo: conversa do WhatsApp Web, e-mail
/// baixado para uso offline, arquivo do Figma, progresso de jogo. Numa máquina
/// real ele tinha 1,7 GB — de longe a maior pasta do perfil, e o alvo óbvio de
/// quem varre por tamanho. Apagar desloga a pessoa de tudo e destrói dado que
/// não está em lugar nenhum.
///
/// Isto aqui é medido e mostrado, e nunca oferecido para limpeza.
const DADO_DE_APLICATIVO: &[&str] = &["IndexedDB", "Local Storage", "Local Extension Settings"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub version: String,
    pub size_mb: f64,
    /// Quantas permissões a extensão pede. Número alto não é acusação, mas é
    /// informação que ninguém dá ao usuário.
    pub permissions: usize,
    /// `None` quando não foi possível determinar. Não vira "false" por padrão:
    /// acusar instalação fora da loja sem certeza seria difamar um programa.
    pub from_webstore: Option<bool>,
    /// Versões antigas da mesma extensão que ficaram em disco sem uso.
    pub stale_versions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserProfile {
    pub name: String,
    pub extensions: Vec<Extension>,
    /// Bytes que dá para apagar com segurança.
    pub cache_bytes: u64,
    /// Bytes de dado de aplicativo. Medido para informar, nunca para limpar.
    pub app_data_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub name: String,
    pub executable: String,
    pub is_default: bool,
    pub running: bool,
    /// Memória somada de todos os processos deste navegador.
    pub ram_mb: f64,
    pub profiles: Vec<BrowserProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserReport {
    pub browsers: Vec<BrowserInfo>,
    pub total_cache_mb: f64,
    pub total_app_data_mb: f64,
    pub total_ram_mb: f64,
    /// Fatia da memória da máquina que os navegadores estão ocupando agora.
    pub ram_percent: f64,
    pub total_extensions: usize,
    pub note: String,
}

// ------------------------------------------------------ onde os perfis moram

/// Navegadores conhecidos: nome, executável e caminho da pasta de dados.
///
/// O caminho é relativo à pasta local do usuário, exceto onde indicado.
fn navegadores_conhecidos() -> Vec<(&'static str, &'static str, PathBuf)> {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let base = PathBuf::from(local);

    vec![
        (
            "Google Chrome",
            "chrome.exe",
            base.join(r"Google\Chrome\User Data"),
        ),
        (
            "Microsoft Edge",
            "msedge.exe",
            base.join(r"Microsoft\Edge\User Data"),
        ),
        (
            "Brave",
            "brave.exe",
            base.join(r"BraveSoftware\Brave-Browser\User Data"),
        ),
        ("Vivaldi", "vivaldi.exe", base.join(r"Vivaldi\User Data")),
        (
            "Opera",
            "opera.exe",
            base.join(r"Programs\Opera\User Data"),
        ),
    ]
}

/// Perfis dentro da pasta de dados de um navegador.
///
/// O critério é a presença do arquivo `Preferences`, e não o nome da pasta.
/// Filtrar por "Default" e "Profile N" perderia perfis renomeados e ainda
/// deixaria passar `System Profile`, `ShaderCache` e `GrShaderCache`, que não
/// são perfis de ninguém.
pub fn e_perfil(dir: &Path) -> bool {
    dir.is_dir() && dir.join("Preferences").is_file()
}

fn perfis(user_data: &Path) -> Vec<PathBuf> {
    let Ok(entradas) = std::fs::read_dir(user_data) else {
        return Vec::new();
    };

    let mut achados: Vec<PathBuf> = entradas
        .flatten()
        .map(|e| e.path())
        .filter(|p| e_perfil(p))
        .collect();

    achados.sort();
    achados
}

// ------------------------------------------------------------ tamanho em disco

/// Soma recursiva, sem seguir link e sem olhar conteúdo de arquivo.
fn somar_pasta(dir: &Path) -> u64 {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return 0;
    };

    entradas
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok()?;

            if meta.is_dir() {
                Some(somar_pasta(&e.path()))
            } else {
                Some(meta.len())
            }
        })
        .sum()
}

fn somar_categorias(perfil: &Path, categorias: &[&str]) -> u64 {
    categorias
        .iter()
        .map(|c| somar_pasta(&perfil.join(c)))
        .sum::<u64>()
        // O cache de service worker fica numa subpasta, e só o de script é
        // descartável: `CacheStorage` guarda resposta que o site pediu para
        // manter offline.
        + somar_pasta(&perfil.join("Service Worker").join("ScriptCache"))
}

// -------------------------------------------------- nome legível da extensão

#[derive(Debug, Deserialize, Default)]
struct Manifest {
    name: Option<String>,
    version: Option<String>,
    default_locale: Option<String>,
    permissions: Option<Vec<serde_json::Value>>,
    host_permissions: Option<Vec<serde_json::Value>>,
}

/// Extrai a chave de um nome no formato `__MSG_chave__`.
pub fn chave_de_traducao(nome: &str) -> Option<&str> {
    nome.strip_prefix("__MSG_")?.strip_suffix("__")
}

/// Procura a tradução de uma chave num `messages.json` já lido.
///
/// A busca é insensível a maiúsculas de propósito: a especificação do Chrome
/// diz que a chave é insensível, e existem extensões que escrevem `extname` no
/// manifesto e `extName` no arquivo de mensagens. Comparar exato perderia essas.
pub fn traduzir_no_json(conteudo: &str, chave: &str) -> Option<String> {
    let raiz: serde_json::Value = serde_json::from_str(conteudo).ok()?;
    let objeto = raiz.as_object()?;

    let alvo = chave.to_lowercase();

    objeto
        .iter()
        .find(|(k, _)| k.to_lowercase() == alvo)
        .and_then(|(_, v)| v.get("message"))
        .and_then(|m| m.as_str())
        .map(|s| s.to_string())
}

/// Ordem de idiomas em que a tradução é procurada.
///
/// Português primeiro porque é o público do produto; depois o idioma que a
/// própria extensão declara como padrão; depois inglês. Sem isso, um terço das
/// extensões de uma máquina real apareceria na tela como `__MSG_appName__`.
fn ordem_de_locales(default_locale: Option<&str>) -> Vec<String> {
    let mut ordem = vec!["pt_BR".to_string(), "pt".to_string()];

    if let Some(padrao) = default_locale {
        ordem.push(padrao.to_string());
    }

    ordem.push("en_US".to_string());
    ordem.push("en".to_string());
    ordem
}

/// Resolve o nome de exibição de uma extensão.
///
/// Quando nada resolve, devolve o id. Inventar um nome seria pior que mostrar
/// o identificador cru: o técnico consegue pesquisar o id, não consegue
/// desfazer um palpite.
fn nome_legivel(versao_dir: &Path, manifesto: &Manifest, id: &str) -> String {
    let bruto = manifesto.name.clone().unwrap_or_default();

    let Some(chave) = chave_de_traducao(&bruto) else {
        return if bruto.trim().is_empty() {
            id.to_string()
        } else {
            bruto
        };
    };

    let locales = versao_dir.join("_locales");

    for candidato in ordem_de_locales(manifesto.default_locale.as_deref()) {
        let arquivo = locales.join(&candidato).join("messages.json");

        if let Ok(conteudo) = std::fs::read_to_string(&arquivo) {
            if let Some(nome) = traduzir_no_json(&conteudo, chave) {
                return nome;
            }
        }
    }

    // Última tentativa: qualquer idioma que exista. Um nome em alemão é melhor
    // que `__MSG_extName__` na tela do cliente.
    if let Ok(entradas) = std::fs::read_dir(&locales) {
        for entrada in entradas.flatten() {
            let arquivo = entrada.path().join("messages.json");

            if let Ok(conteudo) = std::fs::read_to_string(&arquivo) {
                if let Some(nome) = traduzir_no_json(&conteudo, chave) {
                    return nome;
                }
            }
        }
    }

    id.to_string()
}

/// A versão instalada mais recente de uma extensão, e quantas sobraram atrás.
///
/// O Chromium deixa versões antigas em disco depois de atualizar. Elas não
/// rodam e continuam ocupando espaço — é lixo que ninguém mostra ao usuário.
fn versao_ativa(dir_extensao: &Path) -> Option<(PathBuf, usize)> {
    let mut versoes: Vec<PathBuf> = std::fs::read_dir(dir_extensao)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("manifest.json").is_file())
        .collect();

    if versoes.is_empty() {
        return None;
    }

    versoes.sort();
    let ativa = versoes.pop()?;

    Some((ativa, versoes.len()))
}

fn ler_extensoes(perfil: &Path) -> Vec<Extension> {
    let pasta = perfil.join("Extensions");

    let Ok(entradas) = std::fs::read_dir(&pasta) else {
        return Vec::new();
    };

    let mut lista: Vec<Extension> = entradas
        .flatten()
        .filter_map(|entrada| {
            let dir = entrada.path();
            let id = dir.file_name()?.to_string_lossy().to_string();

            // Id de extensão do Chromium tem 32 letras minúsculas. O filtro
            // evita tratar pasta de apoio como extensão.
            if id.len() != 32 || !id.chars().all(|c| c.is_ascii_lowercase()) {
                return None;
            }

            let (versao_dir, stale_versions) = versao_ativa(&dir)?;
            let conteudo = std::fs::read_to_string(versao_dir.join("manifest.json")).ok()?;
            let manifesto: Manifest = serde_json::from_str(&conteudo).ok()?;

            let permissions = manifesto.permissions.as_ref().map_or(0, |p| p.len())
                + manifesto.host_permissions.as_ref().map_or(0, |p| p.len());

            Some(Extension {
                name: nome_legivel(&versao_dir, &manifesto, &id),
                version: manifesto.version.clone().unwrap_or_default(),
                size_mb: somar_pasta(&dir) as f64 / 1_048_576.0,
                permissions,
                from_webstore: None,
                stale_versions,
                id,
            })
        })
        .collect();

    lista.sort_by(|a, b| b.size_mb.total_cmp(&a.size_mb));
    lista
}

// ------------------------------------------------------------------ memória

/// Memória por navegador, somando todos os processos de cada um.
///
/// Por navegador, e não por extensão: ver o cabeçalho do arquivo.
fn memoria_por_executavel() -> std::collections::HashMap<String, f64> {
    use sysinfo::System;

    let mut sistema = System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut total: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

    for processo in sistema.processes().values() {
        let nome = processo.name().to_string_lossy().to_lowercase();
        *total.entry(nome).or_insert(0.0) += processo.memory() as f64 / 1_048_576.0;
    }

    total
}

/// Executável do navegador padrão do sistema.
///
/// Sai do registro pelo `ProgId`, e a identificação é feita pelo nome do
/// executável. O nome amigável — "Microsoft Edge HTML Document" — vem traduzido
/// e não serve para comparar.
pub fn navegador_padrao() -> Option<String> {
    let progid = super::registry::read_text(
        "HKCU",
        r"SOFTWARE\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice",
        "ProgId",
    )?;

    let comando = super::registry::read_text(
        "HKCR",
        &format!(r"{}\shell\open\command", progid),
        "",
    )?;

    let minusculo = comando.to_lowercase();

    ["chrome.exe", "msedge.exe", "brave.exe", "vivaldi.exe", "opera.exe", "firefox.exe"]
        .iter()
        .find(|exe| minusculo.contains(*exe))
        .map(|exe| exe.to_string())
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1_048_576.0
}

/// Monta a frase de resumo.
///
/// Ela precisa dizer as duas contrapartidas: que limpar cache deixa o primeiro
/// carregamento mais lento, e que o dado de aplicativo não é lixo.
pub fn montar_nota(cache_mb: f64, app_data_mb: f64, algum_aberto: bool) -> String {
    let mut nota = String::new();

    if cache_mb >= 1.0 {
        nota.push_str(&format!(
            "{:.0} MB de cache dá para apagar. Vale saber a contrapartida: os sites que \
             você usa vão carregar mais devagar na primeira vez depois da limpeza, porque \
             precisam baixar tudo de novo. ",
            cache_mb
        ));
    }

    if app_data_mb >= 1.0 {
        nota.push_str(&format!(
            "Outros {:.0} MB são dado de aplicativo — conversa de WhatsApp Web, e-mail \
             guardado para uso sem internet, arquivo de editor online. Parece cache pelo \
             tamanho e não é: apagar desloga você de tudo e o que estiver ali some. O \
             Otimiza mede e mostra, mas não oferece limpar. ",
            app_data_mb
        ));
    }

    if algum_aberto {
        nota.push_str(
            "Feche o navegador antes de limpar: com ele aberto os arquivos ficam travados.",
        );
    }

    if nota.is_empty() {
        nota.push_str("Nenhum navegador conhecido encontrado com dados nesta máquina.");
    }

    nota
}

/// Análise completa dos navegadores.
pub fn analyze() -> BrowserReport {
    let memoria = memoria_por_executavel();
    let padrao = navegador_padrao();

    let mut browsers = Vec::new();
    let mut total_cache = 0u64;
    let mut total_app_data = 0u64;
    let mut total_ram = 0.0f64;
    let mut total_extensions = 0usize;
    let mut algum_aberto = false;

    for (nome, executavel, user_data) in navegadores_conhecidos() {
        if !user_data.is_dir() {
            continue;
        }

        let ram_mb = memoria.get(executavel).copied().unwrap_or(0.0);
        let running = ram_mb > 0.0;
        algum_aberto |= running;
        total_ram += ram_mb;

        let mut lista_perfis = Vec::new();

        for caminho in perfis(&user_data) {
            let cache_bytes = somar_categorias(&caminho, CACHE_DESCARTAVEL);
            let app_data_bytes = somar_categorias(&caminho, DADO_DE_APLICATIVO);
            let extensions = ler_extensoes(&caminho);

            total_cache += cache_bytes;
            total_app_data += app_data_bytes;
            total_extensions += extensions.len();

            lista_perfis.push(BrowserProfile {
                name: caminho
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                extensions,
                cache_bytes,
                app_data_bytes,
            });
        }

        browsers.push(BrowserInfo {
            is_default: padrao.as_deref() == Some(executavel),
            name: nome.to_string(),
            executable: executavel.to_string(),
            running,
            ram_mb,
            profiles: lista_perfis,
        });
    }

    // Quem está consumindo mais primeiro.
    browsers.sort_by(|a, b| b.ram_mb.total_cmp(&a.ram_mb));

    let ram_total_maquina = super::hardware::profile().total_ram_gb * 1024.0;
    let ram_percent = if ram_total_maquina > 0.0 {
        total_ram / ram_total_maquina * 100.0
    } else {
        0.0
    };

    BrowserReport {
        note: montar_nota(mb(total_cache), mb(total_app_data), algum_aberto),
        browsers,
        total_cache_mb: mb(total_cache),
        total_app_data_mb: mb(total_app_data),
        total_ram_mb: total_ram,
        ram_percent,
        total_extensions,
    }
}

// ------------------------------------------------------------------ limpeza

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanOutcome {
    pub freed_mb: f64,
    pub message: String,
}

/// Apaga o cache descartável de um navegador.
///
/// Três travas, e nenhuma delas é opcional:
///
/// 1. O navegador precisa estar FECHADO. Com ele aberto os arquivos estão
///    travados e a limpeza sairia pela metade, sem aviso.
/// 2. Só as pastas de `CACHE_DESCARTAVEL`. `IndexedDB` e `Local Storage` não
///    passam nem por engano — ver a constante e o comentário dela.
/// 3. Apaga o CONTEÚDO, não a pasta. Remover a pasta faz parte dos navegadores
///    reclamar de perfil corrompido no próximo início.
///
/// Isto não tem volta, e a interface precisa dizer isso antes.
pub fn limpar_cache(executavel: &str) -> Result<CleanOutcome, String> {
    let memoria = memoria_por_executavel();

    if memoria.get(executavel).copied().unwrap_or(0.0) > 0.0 {
        return Err(format!(
            "O navegador está aberto. Feche-o por completo antes de limpar — com ele \
             rodando, os arquivos ficam travados e a limpeza sairia pela metade."
        ));
    }

    let alvo = navegadores_conhecidos()
        .into_iter()
        .find(|(_, exe, _)| *exe == executavel)
        .ok_or_else(|| format!("Navegador `{}` não é conhecido pelo Otimiza.", executavel))?;

    let (nome, _, user_data) = alvo;

    if !user_data.is_dir() {
        return Err(format!("`{}` não está instalado nesta máquina.", nome));
    }

    let mut antes = 0u64;
    let mut depois = 0u64;

    for perfil in perfis(&user_data) {
        antes += somar_categorias(&perfil, CACHE_DESCARTAVEL);

        for categoria in CACHE_DESCARTAVEL {
            esvaziar(&perfil.join(categoria));
        }
        esvaziar(&perfil.join("Service Worker").join("ScriptCache"));

        depois += somar_categorias(&perfil, CACHE_DESCARTAVEL);
    }

    let liberado = antes.saturating_sub(depois);

    Ok(CleanOutcome {
        freed_mb: mb(liberado),
        message: format!(
            "{:.0} MB liberados do {}. Os sites que você usa vão carregar mais devagar \
             na primeira visita, porque precisam baixar tudo de novo.",
            mb(liberado),
            nome
        ),
    })
}

/// Esvazia o conteúdo de uma pasta, mantendo a pasta.
///
/// Arquivo travado é pulado em silêncio de propósito: a conferência do que
/// sobrou é feita medindo de novo depois, então o número relatado é o que foi
/// realmente apagado, e não o que se esperava apagar.
fn esvaziar(dir: &Path) {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return;
    };

    for entrada in entradas.flatten() {
        let caminho = entrada.path();

        let _ = if caminho.is_dir() {
            std::fs::remove_dir_all(&caminho)
        } else {
            std::fs::remove_file(&caminho)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chave_de_traducao_e_extraida() {
        assert_eq!(chave_de_traducao("__MSG_extName__"), Some("extName"));
        assert_eq!(chave_de_traducao("__MSG_appName__"), Some("appName"));
        // Nome normal não é chave.
        assert_eq!(chave_de_traducao("Google Docs Offline"), None);
        assert_eq!(chave_de_traducao("__MSG_"), None);
    }

    #[test]
    fn traducao_ignora_maiusculas_da_chave() {
        // A especificação do Chrome diz que a chave é insensível a maiúsculas,
        // e existem extensões que escrevem `extname` no manifesto e `extName`
        // no arquivo de mensagens. Comparar exato perderia essas.
        let json = r#"{ "extName": { "message": "Bloqueador de Anúncios" } }"#;

        assert_eq!(
            traduzir_no_json(json, "extname").as_deref(),
            Some("Bloqueador de Anúncios")
        );
        assert_eq!(
            traduzir_no_json(json, "extName").as_deref(),
            Some("Bloqueador de Anúncios")
        );
        assert_eq!(traduzir_no_json(json, "outra"), None);
    }

    #[test]
    fn acento_sobrevive_a_traducao() {
        // O nome vai para a tela e para o relatório do cliente.
        let json = r#"{ "n": { "message": "Tradução — versãoção" } }"#;
        assert_eq!(
            traduzir_no_json(json, "n").as_deref(),
            Some("Tradução — versãoção")
        );
    }

    #[test]
    fn portugues_vem_antes_do_ingles() {
        let ordem = ordem_de_locales(Some("de"));

        let pos = |s: &str| ordem.iter().position(|x| x == s).unwrap();

        assert!(pos("pt_BR") < pos("en_US"));
        // O idioma declarado pela extensão vem antes do inglês genérico.
        assert!(pos("de") < pos("en_US"));
    }

    #[test]
    fn dado_de_aplicativo_nunca_entra_no_que_se_apaga() {
        // A confusão mais cara possível deste módulo. IndexedDB tinha 1,7 GB
        // numa máquina real — é o alvo óbvio de quem varre por tamanho, e
        // apagar desloga a pessoa de tudo.
        for protegido in DADO_DE_APLICATIVO {
            assert!(
                !CACHE_DESCARTAVEL.contains(protegido),
                "`{}` apareceu na lista do que se apaga",
                protegido
            );
        }

        assert!(DADO_DE_APLICATIVO.contains(&"IndexedDB"));
        assert!(DADO_DE_APLICATIVO.contains(&"Local Storage"));
    }

    #[test]
    fn nota_avisa_as_duas_contrapartidas() {
        let nota = montar_nota(800.0, 1700.0, true);

        // Limpar cache não é ganho puro, e isso precisa estar dito.
        assert!(nota.contains("mais devagar na primeira vez"));
        // E o dado de aplicativo precisa ser explicado, não só listado.
        assert!(nota.contains("desloga você"));
        assert!(nota.contains("não oferece limpar"));
        assert!(nota.contains("Feche o navegador"));
    }

    #[test]
    fn sem_navegador_a_nota_nao_fica_vazia() {
        let nota = montar_nota(0.0, 0.0, false);
        assert!(!nota.is_empty());
    }

    #[test]
    fn perfil_e_reconhecido_pelo_arquivo_e_nao_pelo_nome() {
        // `System Profile`, `ShaderCache` e `GrShaderCache` moram ao lado dos
        // perfis de verdade e não têm `Preferences`.
        let temp = std::env::temp_dir().join("otimiza_teste_perfil");
        let real = temp.join("Perfil Renomeado");
        let falso = temp.join("ShaderCache");

        std::fs::create_dir_all(&real).unwrap();
        std::fs::create_dir_all(&falso).unwrap();
        std::fs::write(real.join("Preferences"), "{}").unwrap();

        assert!(e_perfil(&real), "perfil renomeado precisa ser reconhecido");
        assert!(!e_perfil(&falso), "pasta de apoio não é perfil");

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn navegador_aberto_recusa_limpeza() {
        // Com o navegador rodando os arquivos ficam travados e a limpeza sairia
        // pela metade, sem ninguém perceber.
        let abertos = memoria_por_executavel();

        for (_, executavel, user_data) in navegadores_conhecidos() {
            if !user_data.is_dir() || abertos.get(executavel).copied().unwrap_or(0.0) <= 0.0 {
                continue;
            }

            let erro = limpar_cache(executavel).unwrap_err();
            assert!(erro.contains("Feche-o"), "recusa inesperada: {}", erro);
            return;
        }

        println!("nenhum navegador aberto agora; caso não exercitado");
    }

    #[test]
    fn navegador_desconhecido_e_recusado() {
        let erro = limpar_cache("naoexiste.exe").unwrap_err();
        assert!(erro.contains("não é conhecido"));
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();

        println!(
            "{} navegador(es), {} extensões, {:.0} MB de RAM ({:.1}% da máquina)",
            r.browsers.len(),
            r.total_extensions,
            r.total_ram_mb,
            r.ram_percent
        );
        println!(
            "  cache: {:.0} MB | dado de aplicativo: {:.0} MB",
            r.total_cache_mb, r.total_app_data_mb
        );

        for b in &r.browsers {
            println!(
                "  {} {} — {:.0} MB, {} perfil(is){}",
                b.name,
                if b.running { "(aberto)" } else { "(fechado)" },
                b.ram_mb,
                b.profiles.len(),
                if b.is_default { " [padrão]" } else { "" }
            );

            for p in &b.profiles {
                for e in p.extensions.iter().take(4) {
                    println!(
                        "      {} v{} — {:.2} MB, {} permissões{}",
                        e.name,
                        e.version,
                        e.size_mb,
                        e.permissions,
                        if e.stale_versions > 0 {
                            format!(", {} versão(ões) antiga(s) em disco", e.stale_versions)
                        } else {
                            String::new()
                        }
                    );
                }
            }
        }

        assert!(!r.note.is_empty());

        // Nenhuma extensão pode chegar à tela com a chave crua de tradução.
        // Um terço delas apareceria assim sem a resolução de idioma.
        for b in &r.browsers {
            for p in &b.profiles {
                for e in &p.extensions {
                    assert!(
                        !e.name.contains("__MSG_"),
                        "extensão com nome não resolvido: {}",
                        e.name
                    );
                    assert!(!e.name.is_empty());
                }
            }
        }

        // Do que mais consome para o que menos consome.
        assert!(r.browsers.windows(2).all(|p| p[0].ram_mb >= p[1].ram_mb));
        assert!(r.ram_percent >= 0.0 && r.ram_percent <= 100.0);
    }
}
