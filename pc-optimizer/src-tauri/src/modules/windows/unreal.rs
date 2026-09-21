// A configuração gráfica dos jogos em Unreal Engine — um ajustador para todos
//
// POR QUE ISTO EXISTE
//
// Até a 2.7 o Otimiza só sabia mexer na configuração de UM jogo (GTA V/FiveM,
// `configjogo.rs`). A maior parte dos jogos atuais roda em Unreal Engine 4/5
// — Fortnite, Valorant, PUBG, Marvel Rivals, e boa parte dos lançamentos — e
// todos guardam a qualidade escolhida pelo jogador no mesmo formato. Um
// ajustador só, com teste, cobre todos eles.
//
// O QUE A DOCUMENTAÇÃO DA EPIC DIZ, E É SÓ ISSO QUE ESTE MÓDULO USA
//
// - A escolha do jogador fica no `GameUserSettings.ini` do aparelho: a classe
//   UGameUserSettings "guarda a qualidade escolhida para cada grupo de
//   escalabilidade" (Customizing Device Profiles and Scalability).
// - Os grupos são variáveis `sg.*`, de 0 (Baixo) a 3 (Épico): "The Low
//   setting equates to sg.ShadowQuality 0 and Epic to sg.ShadowQuality 3"
//   (Scalability Reference). Alguns jogos gravam 4 (Cinematográfico).
//
// AS REGRAS DO AJUSTADOR
//
// 1. Só mexe em linha `sg.Algo=N` que JÁ EXISTE dentro de `[ScalabilityGroups]`.
//    Arquivo sem essa seção = formato que não reconhecemos = recusa.
// 2. Só DESCE valor, nunca sobe. Quem já joga no baixo continua no baixo.
// 3. Não mexe em resolução, distância de visão (é jogabilidade: ver inimigo
//    longe) nem em limite de FPS (a regra da 2.9: nunca pôr teto).
// 4. Jogo aberto = recusa: o jogo regrava o arquivo ao fechar e apagaria a
//    mudança, fazendo o cliente achar que aplicou.
// 5. O arquivo inteiro vai para o histórico antes; o desfazer devolve byte a
//    byte (o mesmo `ChangeRecord::GameConfig` do FiveM).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Quanto de imagem a pessoa aceita trocar por FPS. É a ÚNICA escolha: não
/// existe "perfil de RP" nem "perfil competitivo" — todo mundo ganha FPS, e
/// a diferença é só quanto visual cada um topa perder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orcamento {
    MaxFps,
    Equilibrado,
    Qualidade,
}

/// Teto de cada grupo por orçamento. `None` = não mexe.
///
/// Sombra, pós-processamento, efeitos, folhagem, iluminação global e reflexo
/// são o que mais custa em qualquer motor. Textura quase não custa FPS — custa
/// VRAM, e por isso só desce quando a placa tem pouca memória.
fn teto(grupo: &str, orc: Orcamento, vram_gb: Option<f64>) -> Option<u8> {
    let g = grupo.to_ascii_lowercase();
    let pouca_vram = vram_gb.map(|v| v <= 4.5).unwrap_or(false);
    let muito_pouca_vram = vram_gb.map(|v| v <= 2.5).unwrap_or(false);
    match (g.as_str(), orc) {
        ("sg.shadowquality", Orcamento::MaxFps) => Some(0),
        ("sg.shadowquality", Orcamento::Equilibrado) => Some(1),
        ("sg.shadowquality", Orcamento::Qualidade) => Some(2),
        ("sg.postprocessquality", Orcamento::MaxFps) => Some(0),
        ("sg.postprocessquality", Orcamento::Equilibrado) => Some(1),
        ("sg.postprocessquality", Orcamento::Qualidade) => Some(2),
        ("sg.effectsquality", Orcamento::MaxFps) => Some(0),
        ("sg.effectsquality", _) => Some(2),
        ("sg.foliagequality", Orcamento::MaxFps) => Some(0),
        ("sg.foliagequality", Orcamento::Equilibrado) => Some(1),
        ("sg.foliagequality", Orcamento::Qualidade) => Some(2),
        ("sg.globalilluminationquality", Orcamento::MaxFps) => Some(0),
        ("sg.globalilluminationquality", Orcamento::Equilibrado) => Some(1),
        ("sg.globalilluminationquality", Orcamento::Qualidade) => Some(2),
        ("sg.reflectionquality", Orcamento::MaxFps) => Some(0),
        ("sg.reflectionquality", Orcamento::Equilibrado) => Some(1),
        ("sg.reflectionquality", Orcamento::Qualidade) => Some(2),
        ("sg.shadingquality", Orcamento::MaxFps) => Some(1),
        ("sg.shadingquality", _) => Some(2),
        ("sg.texturequality", _) if muito_pouca_vram => Some(1),
        ("sg.texturequality", Orcamento::Qualidade) => None,
        ("sg.texturequality", _) if pouca_vram => Some(2),
        // Resolução, distância de visão e anti-serrilhado: fora de propósito.
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mudanca {
    pub chave: String,
    pub antes: u8,
    pub depois: u8,
}

/// Aplica o orçamento no texto do `GameUserSettings.ini`. **Função pura.**
///
/// `None` quando o arquivo não tem `[ScalabilityGroups]` — formato que não
/// reconhecemos, e aí não se escreve nada. Preserva quebras de linha,
/// espaços, comentários e tudo que não é `sg.*` dentro da seção.
pub fn aplicar_no_texto(texto: &str, orc: Orcamento, vram_gb: Option<f64>) -> Option<(String, Vec<Mudanca>)> {
    let mut na_secao = false;
    let mut achou_secao = false;
    let mut mudancas = Vec::new();
    let mut saida = String::with_capacity(texto.len());

    for pedaco in texto.split_inclusive('\n') {
        let (linha, fim) = match pedaco.strip_suffix("\r\n") {
            Some(l) => (l, "\r\n"),
            None => match pedaco.strip_suffix('\n') {
                Some(l) => (l, "\n"),
                None => (pedaco, ""),
            },
        };
        let t = linha.trim();
        if t.starts_with('[') && t.ends_with(']') {
            na_secao = t.eq_ignore_ascii_case("[ScalabilityGroups]");
            achou_secao |= na_secao;
        }
        let mut nova = None;
        if na_secao {
            if let Some((chave, valor)) = t.split_once('=') {
                let chave = chave.trim();
                if chave.to_ascii_lowercase().starts_with("sg.") {
                    if let (Ok(atual), Some(max)) = (valor.trim().parse::<u8>(), teto(chave, orc, vram_gb)) {
                        if atual > max {
                            mudancas.push(Mudanca { chave: chave.to_string(), antes: atual, depois: max });
                            nova = Some(format!("{}={}", chave, max));
                        }
                    }
                }
            }
        }
        saida.push_str(nova.as_deref().unwrap_or(linha));
        saida.push_str(fim);
    }
    achou_secao.then_some((saida, mudancas))
}

// ------------------------------------------------------------ achar o arquivo

/// Nomes que podem ser a pasta do jogo em `%LOCALAPPDATA%`, a partir do
/// executável. **Função pura.**
///
/// Layout do Unreal: `...\<Projeto>\Binaries\Win64\<Nome>-Win64-Shipping.exe`.
/// A pasta de configuração usa às vezes o projeto (`FortniteGame`, `TslGame`)
/// e às vezes o nome do executável (`VALORANT`), então os dois entram.
pub fn candidatos(executavel: &Path) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(stem) = executavel.file_stem().and_then(|s| s.to_str()) {
        let base = stem.split("-Win64").next().unwrap_or(stem);
        v.push(base.to_string());
        if let Some(sem_client) = base.strip_suffix("Client") {
            v.push(format!("{}Game", sem_client));
            v.push(sem_client.to_string());
        }
    }
    let partes: Vec<String> = executavel
        .components()
        .filter_map(|c| c.as_os_str().to_str().map(str::to_string))
        .collect();
    if let Some(i) = partes.iter().position(|p| p.eq_ignore_ascii_case("Binaries")) {
        if i >= 1 {
            v.push(partes[i - 1].clone());
        }
        if i >= 2 {
            v.push(partes[i - 2].clone());
        }
    }
    v.retain(|s| !s.is_empty());
    v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    v
}

/// É layout de Unreal? (`Binaries\Win64` no caminho, ou `-Win64-Shipping`.)
pub fn parece_unreal(executavel: &Path) -> bool {
    let s = executavel.to_string_lossy().to_ascii_lowercase();
    s.contains("\\binaries\\win64\\") || s.contains("-win64-shipping")
}

/// Procura `GameUserSettings.ini` sob `<pasta>\Saved\Config\` até três níveis.
fn configs_em(pasta: &Path) -> Vec<PathBuf> {
    fn descer(p: &Path, nivel: u8, v: &mut Vec<PathBuf>) {
        let Ok(entradas) = std::fs::read_dir(p) else { return };
        for e in entradas.flatten() {
            let c = e.path();
            if c.is_dir() && nivel < 3 {
                descer(&c, nivel + 1, v);
            } else if c.file_name().map(|n| n.eq_ignore_ascii_case("GameUserSettings.ini")).unwrap_or(false) {
                v.push(c);
            }
        }
    }
    let mut v = Vec::new();
    descer(&pasta.join("Saved").join("Config"), 0, &mut v);
    v
}

/// O `GameUserSettings.ini` deste jogo, se houver. Com mais de um (contas
/// diferentes, como no Valorant), o modificado por último — é o da conta
/// que jogou por último.
pub fn config_do_jogo(executavel: &Path) -> Option<PathBuf> {
    config_do_jogo_em(&PathBuf::from(std::env::var("LOCALAPPDATA").ok()?), executavel)
}

/// O mesmo, com a pasta de dados por parâmetro (para teste).
pub fn config_do_jogo_em(local: &Path, executavel: &Path) -> Option<PathBuf> {
    let mut achados: Vec<PathBuf> = Vec::new();
    for nome in candidatos(executavel) {
        let pasta = local.join(&nome);
        if pasta.is_dir() {
            achados.extend(configs_em(&pasta));
        }
    }
    achados
        .into_iter()
        .filter(|c| std::fs::read_to_string(c).map(|t| t.to_ascii_lowercase().contains("[scalabilitygroups]")).unwrap_or(false))
        .max_by_key(|c| std::fs::metadata(c).and_then(|m| m.modified()).ok())
}

// ------------------------------------------------------------ prever e aplicar

#[derive(Debug, Clone, Serialize)]
pub struct Previa {
    pub arquivo: PathBuf,
    pub mudancas: Vec<Mudanca>,
}

pub fn prever(executavel: &Path, orc: Orcamento, vram_gb: Option<f64>) -> Result<Previa, String> {
    let arquivo = config_do_jogo(executavel)
        .ok_or("Não achei a configuração gráfica deste jogo. Abra o jogo uma vez, mude qualquer opção gráfica e feche — o jogo cria o arquivo.")?;
    let texto = std::fs::read_to_string(&arquivo).map_err(|e| format!("Não consegui ler {}: {}", arquivo.display(), e))?;
    let (_, mudancas) = aplicar_no_texto(&texto, orc, vram_gb).ok_or("O arquivo não está no formato que o Otimiza reconhece; nada foi alterado.")?;
    Ok(Previa { arquivo, mudancas })
}

pub struct Feito {
    pub arquivo: PathBuf,
    pub anterior: String,
    pub mudancas: Vec<Mudanca>,
}

/// Grava. Quem chama registra `anterior` no histórico (ver `commands.rs`).
pub fn aplicar(executavel: &Path, orc: Orcamento, vram_gb: Option<f64>) -> Result<Feito, String> {
    if let Some(nome) = executavel.file_name().and_then(|n| n.to_str()) {
        if jogo_rodando(nome) {
            return Err("Feche o jogo antes: ele regrava a configuração ao fechar e apagaria a mudança.".into());
        }
    }
    let arquivo = config_do_jogo(executavel).ok_or("Não achei a configuração gráfica deste jogo.")?;
    aplicar_no_arquivo(&arquivo, orc, vram_gb)
}

/// Grava num arquivo já localizado.
pub fn aplicar_no_arquivo(arquivo: &Path, orc: Orcamento, vram_gb: Option<f64>) -> Result<Feito, String> {
    let arquivo = arquivo.to_path_buf();
    let meta = std::fs::metadata(&arquivo).map_err(|e| e.to_string())?;
    if meta.permissions().readonly() {
        return Err("O arquivo de configuração está marcado como somente leitura — alguém travou de propósito. Nada foi alterado.".into());
    }
    let anterior = std::fs::read_to_string(&arquivo).map_err(|e| e.to_string())?;
    let (novo, mudancas) = aplicar_no_texto(&anterior, orc, vram_gb).ok_or("Formato não reconhecido; nada foi alterado.")?;
    if !mudancas.is_empty() {
        let temporario = arquivo.with_extension("ini.otimiza-tmp");
        std::fs::write(&temporario, &novo).map_err(|e| e.to_string())?;
        std::fs::rename(&temporario, &arquivo).map_err(|e| {
            let _ = std::fs::remove_file(&temporario);
            e.to_string()
        })?;
    }
    Ok(Feito { arquivo, anterior, mudancas })
}

fn jogo_rodando(nome_do_exe: &str) -> bool {
    let mut s = sysinfo::System::new();
    s.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    s.processes().values().any(|p| p.name().to_string_lossy().eq_ignore_ascii_case(nome_do_exe))
}

#[cfg(test)]
mod testes {
    use super::*;

    const INI: &str = "[/Script/Engine.GameUserSettings]\r\nbUseVSync=False\r\nFrameRateLimit=0.000000\r\n\r\n[ScalabilityGroups]\r\nsg.ResolutionQuality=100\r\nsg.ViewDistanceQuality=3\r\nsg.AntiAliasingQuality=3\r\nsg.ShadowQuality=3\r\nsg.GlobalIlluminationQuality=3\r\nsg.ReflectionQuality=3\r\nsg.PostProcessQuality=4\r\nsg.TextureQuality=3\r\nsg.EffectsQuality=3\r\nsg.FoliageQuality=3\r\nsg.ShadingQuality=3\r\n\r\n[/Script/FortniteGame.FortGameUserSettings]\r\nsg.ShadowQuality=3\r\n";

    #[test]
    fn equilibrado_desce_o_que_custa_e_nao_toca_no_resto() {
        let (novo, m) = aplicar_no_texto(INI, Orcamento::Equilibrado, Some(8.0)).unwrap();
        assert!(novo.contains("sg.ShadowQuality=1\r\n"));
        assert!(novo.contains("sg.PostProcessQuality=1\r\n"));
        assert!(novo.contains("sg.EffectsQuality=2\r\n"));
        // Fora de propósito:
        assert!(novo.contains("sg.ResolutionQuality=100\r\n"));
        assert!(novo.contains("sg.ViewDistanceQuality=3\r\n"));
        assert!(novo.contains("sg.AntiAliasingQuality=3\r\n"));
        assert!(novo.contains("FrameRateLimit=0.000000\r\n"));
        // Textura com 8 GB de VRAM: não mexe.
        assert!(novo.contains("sg.TextureQuality=3\r\n"));
        // Só dentro de [ScalabilityGroups].
        assert!(novo.ends_with("[/Script/FortniteGame.FortGameUserSettings]\r\nsg.ShadowQuality=3\r\n"));
        assert!(m.iter().all(|x| x.depois < x.antes));
        assert_eq!(novo.len() - novo.replace("\r\n", "").len(), INI.len() - INI.replace("\r\n", "").len());
    }

    #[test]
    fn nunca_sobe_valor() {
        let baixo = "[ScalabilityGroups]\nsg.ShadowQuality=0\nsg.PostProcessQuality=1\n";
        let (novo, m) = aplicar_no_texto(baixo, Orcamento::Qualidade, None).unwrap();
        assert_eq!(novo, baixo);
        assert!(m.is_empty());
    }

    #[test]
    fn aplicar_duas_vezes_da_o_mesmo_arquivo() {
        let (uma, _) = aplicar_no_texto(INI, Orcamento::MaxFps, Some(4.0)).unwrap();
        let (duas, m) = aplicar_no_texto(&uma, Orcamento::MaxFps, Some(4.0)).unwrap();
        assert_eq!(uma, duas);
        assert!(m.is_empty());
    }

    #[test]
    fn pouca_vram_desce_textura() {
        let (novo, _) = aplicar_no_texto(INI, Orcamento::Equilibrado, Some(4.0)).unwrap();
        assert!(novo.contains("sg.TextureQuality=2\r\n"));
        let (novo, _) = aplicar_no_texto(INI, Orcamento::Qualidade, Some(2.0)).unwrap();
        assert!(novo.contains("sg.TextureQuality=1\r\n"));
    }

    #[test]
    fn arquivo_sem_a_secao_e_recusado() {
        assert!(aplicar_no_texto("[Qualquer]\nsg.ShadowQuality=3\n", Orcamento::MaxFps, None).is_none());
        assert!(aplicar_no_texto("", Orcamento::MaxFps, None).is_none());
    }

    #[test]
    fn acha_aplica_e_desfaz_byte_a_byte() {
        use crate::modules::changelog::ChangeRecord;
        let local = std::env::temp_dir().join(format!("otimiza-unreal-{}", std::process::id()));
        let pasta = local.join("VALORANT").join("Saved").join("Config").join("abc123-na").join("Windows");
        std::fs::create_dir_all(&pasta).unwrap();
        let ini = pasta.join("GameUserSettings.ini");
        std::fs::write(&ini, INI).unwrap();
        // Outra pasta de jogo, que não pode ser escolhida.
        let outra = local.join("OutroJogo").join("Saved").join("Config").join("Windows");
        std::fs::create_dir_all(&outra).unwrap();
        std::fs::write(outra.join("GameUserSettings.ini"), INI).unwrap();

        let exe = Path::new(r"C:\Riot Games\VALORANT\live\ShooterGame\Binaries\Win64\VALORANT-Win64-Shipping.exe");
        let achado = config_do_jogo_em(&local, exe).expect("achou");
        assert_eq!(achado, ini);

        let feito = aplicar_no_arquivo(&achado, Orcamento::MaxFps, Some(4.0)).unwrap();
        assert!(!feito.mudancas.is_empty());
        assert_ne!(std::fs::read_to_string(&ini).unwrap(), INI);

        let registro = ChangeRecord::GameConfig {
            caminho: ini.to_string_lossy().to_string(),
            anterior: Some(feito.anterior),
            jogo: "VALORANT".into(),
        };
        crate::modules::windows::revert_changes_para_teste(&[registro]).unwrap();
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), INI);
        let _ = std::fs::remove_dir_all(&local);
    }

    #[test]
    fn candidatos_pelo_executavel() {
        let fortnite = Path::new(r"C:\Epic\Fortnite\FortniteGame\Binaries\Win64\FortniteClient-Win64-Shipping.exe");
        let c = candidatos(fortnite);
        assert!(c.iter().any(|x| x == "FortniteGame"), "{:?}", c);
        let valorant = Path::new(r"C:\Riot Games\VALORANT\live\ShooterGame\Binaries\Win64\VALORANT-Win64-Shipping.exe");
        assert!(candidatos(valorant).iter().any(|x| x == "VALORANT"));
        let pubg = Path::new(r"C:\Steam\steamapps\common\PUBG\TslGame\Binaries\Win64\TslGame.exe");
        assert!(candidatos(pubg).iter().any(|x| x == "TslGame"));
        assert!(parece_unreal(fortnite) && parece_unreal(pubg));
        assert!(!parece_unreal(Path::new(r"C:\Jogos\GTA5.exe")));
    }
}
