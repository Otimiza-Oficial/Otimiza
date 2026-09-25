// O navegador. De fora NÃO dá para medir memória por extensão (várias dividem um processo): o número honesto é
// o do navegador inteiro (as abas pesam; as extensões somavam 39 MB). Lê manifesto, traduções e TAMANHO de pastas;
// nunca abre `History`, `Cookies`, `Login Data`, `Web Data`, `Bookmarks`, `Top Sites` ou `Sessions`, nem para
// contar. Do `IndexedDB` só o total: os nomes das subpastas revelam os sites.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Lista fechada, cada pasta conferida: o critério não é "parece cache pelo nome".
const CACHE_DESCARTAVEL: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "DawnGraphiteCache",
    "DawnWebGPUCache",
    "Shared Dictionary",
];

/// `IndexedDB` guarda dado de aplicativo (WhatsApp Web, e-mail offline, Figma): 1,7 GB numa máquina real, o alvo
/// óbvio de quem varre por tamanho. Apagar desloga de tudo. Medido e mostrado, nunca oferecido para limpeza.
const DADO_DE_APLICATIVO: &[&str] = &["IndexedDB", "Local Storage", "Local Extension Settings"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub version: String,
    pub size_mb: f64,
    pub permissions: usize,
    /// `None` quando não se sabe: acusar instalação fora da loja sem certeza seria difamar um programa.
    pub from_webstore: Option<bool>,
    pub stale_versions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserProfile {
    pub name: String,
    pub extensions: Vec<Extension>,
    pub cache_bytes: u64,
    pub app_data_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub name: String,
    pub executable: String,
    pub is_default: bool,
    pub running: bool,
    pub ram_mb: f64,
    pub profiles: Vec<BrowserProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserReport {
    pub browsers: Vec<BrowserInfo>,
    pub total_cache_mb: f64,
    pub total_app_data_mb: f64,
    pub total_ram_mb: f64,
    pub ram_percent: f64,
    pub total_extensions: usize,
    pub note: String,
}

/// Relativo à pasta local do usuário, exceto onde indicado.
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

/// Pelo arquivo `Preferences`, não pelo nome: perfis renomeados existem, e `System Profile` e `ShaderCache` não
/// são perfis.
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

/// Sem seguir link e sem olhar conteúdo.
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
        // Só o cache de script é descartável: `CacheStorage` guarda o que o site pediu para manter offline.
        + somar_pasta(&perfil.join("Service Worker").join("ScriptCache"))
}

#[derive(Debug, Deserialize, Default)]
struct Manifest {
    name: Option<String>,
    version: Option<String>,
    default_locale: Option<String>,
    permissions: Option<Vec<serde_json::Value>>,
    host_permissions: Option<Vec<serde_json::Value>>,
}

pub fn chave_de_traducao(nome: &str) -> Option<&str> {
    nome.strip_prefix("__MSG_")?.strip_suffix("__")
}

/// Insensível a maiúsculas, como a especificação do Chrome: há `extname` no manifesto e `extName` nas mensagens.
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

/// Português, o padrão da extensão, inglês: sem isso um terço aparecia como `__MSG_appName__`.
fn ordem_de_locales(default_locale: Option<&str>) -> Vec<String> {
    let mut ordem = vec!["pt_BR".to_string(), "pt".to_string()];

    if let Some(padrao) = default_locale {
        ordem.push(padrao.to_string());
    }

    ordem.push("en_US".to_string());
    ordem.push("en".to_string());
    ordem
}

/// Sem tradução, o id: o técnico pesquisa o id, não desfaz um palpite.
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

/// O Chromium deixa as versões antigas em disco depois de atualizar.
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

            // Id de extensão: 32 letras minúsculas.
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

/// Pelo nome do executável: o nome amigável vem traduzido.
pub fn navegador_padrao() -> Option<String> {
    let progid = super::registry::read_text(
        "HKCU",
        r"SOFTWARE\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice",
        "ProgId",
    )
    .ok()
    .flatten()?;

    let comando = super::registry::read_text(
        "HKCR",
        &format!(r"{}\shell\open\command", progid),
        "",
    )
    .ok()
    .flatten()?;

    let minusculo = comando.to_lowercase();

    ["chrome.exe", "msedge.exe", "brave.exe", "vivaldi.exe", "opera.exe", "firefox.exe"]
        .iter()
        .find(|exe| minusculo.contains(*exe))
        .map(|exe| exe.to_string())
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1_048_576.0
}

/// Diz as duas contrapartidas: o primeiro carregamento fica mais lento, e dado de aplicativo não é lixo.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chave_de_traducao_e_extraida() {
        assert_eq!(chave_de_traducao("__MSG_extName__"), Some("extName"));
        assert_eq!(chave_de_traducao("__MSG_appName__"), Some("appName"));
        assert_eq!(chave_de_traducao("Google Docs Offline"), None);
        assert_eq!(chave_de_traducao("__MSG_"), None);
    }

    #[test]
    fn traducao_ignora_maiusculas_da_chave() {
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
        assert!(pos("de") < pos("en_US"));
    }

    #[test]
    fn dado_de_aplicativo_nunca_entra_no_que_se_apaga() {
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

        assert!(nota.contains("mais devagar na primeira vez"));
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

        assert!(r.browsers.windows(2).all(|p| p[0].ram_mb >= p[1].ram_mb));
        assert!(r.ram_percent >= 0.0 && r.ram_percent <= 100.0);
    }
}
