// A configuração do jogo (GTA V / FiveM). Na máquina do dono estava tudo no mínimo e o MSAA em 4x, que sozinho
// custa 30-50% dos quadros numa placa de entrada: a configuração pesa dezenas de por cento, os ajustes de Windows
// somados alguns. O dono tirou a regra "não mexer no jogo"; o risco ficou coberto: a escrita única
// (`aplicar_perfil`) devolve o arquivo INTEIRO; recusa com o jogo aberto; `trocar()` mexe só entre as aspas da
// chave (`toda_escrita_guarda_o_arquivo_anterior` exige as três). `RefreshRate = 60` num monitor de 180 Hz é
// corrigido até no perfil que não mexe no visual.

use super::achados::{FindingSeverity, FixLocation};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AjusteCaro {
    pub chave: String,
    pub valor: String,
    pub onde: String,
    /// Medida por terceiros, nunca por nós.
    pub ganho: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigJogoFinding {
    pub id: String,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: FindingSeverity,
    pub fix_location: FixLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigJogoReport {
    pub arquivo: Option<PathBuf>,
    pub jogo: String,
    pub caros: Vec<AjusteCaro>,
    pub findings: Vec<ConfigJogoFinding>,
}

/// Sem biblioteca de XML: formato rígido gerado pelo jogo.
pub fn valor(conteudo: &str, chave: &str) -> Option<String> {
    let abertura = format!("<{} value=\"", chave);
    let inicio = conteudo.find(&abertura)? + abertura.len();
    let resto = &conteudo[inicio..];
    let fim = resto.find('"')?;

    Some(resto[..fim].to_string())
}

fn numero(conteudo: &str, chave: &str) -> Option<f64> {
    valor(conteudo, chave)?.trim().parse().ok()
}

/// **Pura.** Só os caracteres entre as aspas daquela chave mudam: reescrever o arquivo perderia comentários, ordem,
/// indentação e chaves de versões futuras. `None` sem a chave: inventar a linha escreveria o que o jogo talvez nem
/// leia.
pub fn trocar(conteudo: &str, chave: &str, novo: &str) -> Option<String> {
    let abertura = format!("<{} value=\"", chave);
    let inicio = conteudo.find(&abertura)? + abertura.len();
    let fim = inicio + conteudo[inicio..].find('"')?;

    let mut saida = String::with_capacity(conteudo.len() + novo.len());
    saida.push_str(&conteudo[..inicio]);
    saida.push_str(novo);
    saida.push_str(&conteudo[fim..]);

    Some(saida)
}

/// O critério simples para placa de entrada (uma GTX 1650 tem 4 GB).
const VRAM_DE_PLACA_MODESTA_GB: f64 = 6.0;

/// **Pura.** Apontar MSAA 4x para uma RTX 4070 seria inventar problema.
pub fn diagnosticar(conteudo: &str, vram_gb: f64) -> (Vec<AjusteCaro>, Vec<ConfigJogoFinding>) {
    let mut caros = Vec::new();
    let placa_modesta = vram_gb > 0.0 && vram_gb <= VRAM_DE_PLACA_MODESTA_GB;

    // Primeiro: custa mais que todos os outros juntos, e quem baixa as "qualidades" no menu passa direto por ele.
    if let Some(msaa) = numero(conteudo, "MSAA") {
        if msaa >= 2.0 && placa_modesta {
            caros.push(AjusteCaro {
                chave: "MSAA".to_string(),
                valor: format!("{}x", msaa as u32),
                onde: "Gráficos → MSAA → Desligado".to_string(),
                ganho: "30% a 50%".to_string(),
            });
        }
    }

    if let Some(t) = numero(conteudo, "Tessellation") {
        if t >= 1.0 && placa_modesta {
            caros.push(AjusteCaro {
                chave: "Tessellation".to_string(),
                valor: nivel(t),
                onde: "Gráficos Avançados → Tesselação → Desligada".to_string(),
                ganho: "5% a 10%".to_string(),
            });
        }
    }

    if let Some(s) = numero(conteudo, "SSAO") {
        if s >= 1.0 && placa_modesta {
            caros.push(AjusteCaro {
                chave: "SSAO".to_string(),
                valor: nivel(s),
                onde: "Gráficos → Oclusão de ambiente → Desligada".to_string(),
                ganho: "3% a 8%".to_string(),
            });
        }
    }

    if let Some(q) = numero(conteudo, "ShadowQuality") {
        if q >= 2.0 && placa_modesta {
            caros.push(AjusteCaro {
                chave: "ShadowQuality".to_string(),
                valor: nivel(q),
                onde: "Gráficos → Qualidade das sombras → Normal".to_string(),
                ganho: "5% a 15%".to_string(),
            });
        }
    }

    if let Some(r) = numero(conteudo, "ReflectionQuality") {
        if r >= 2.0 && placa_modesta {
            caros.push(AjusteCaro {
                chave: "ReflectionQuality".to_string(),
                valor: nivel(r),
                onde: "Gráficos → Qualidade dos reflexos → Normal".to_string(),
                ganho: "5% a 12%".to_string(),
            });
        }
    }

    if caros.is_empty() {
        return (caros, Vec::new());
    }

    let lista: Vec<String> = caros
        .iter()
        .map(|c| format!("{} em {}", legivel(&c.chave), c.valor))
        .collect();

    let passos: Vec<String> = caros.iter().map(|c| c.onde.clone()).collect();

    let finding = ConfigJogoFinding {
        id: "config_do_jogo_pesada".to_string(),
        title: "O jogo está pedindo mais da placa do que ela tem".to_string(),
        measured: format!(
            "Com {:.0} GB de memória de vídeo, o jogo está com {}.",
            vram_gb,
            lista.join(", ")
        ),
        advice: format!(
            "Isto pesa muito mais do que qualquer ajuste do Windows. Análises \
             independentes medem entre 30% e 50% de quadros só na suavização de \
             serrilhado — mais do que tudo que este programa consegue fazer no \
             sistema, somado. O Otimiza consegue mudar isto para você, mostrando \
             antes o que muda e guardando o arquivo para desfazer a qualquer \
             momento. Se preferir fazer à mão, no menu do próprio jogo: {}.",
            passos.join("; ")
        ),
        // `Software` porque o botão agora existe; `None` esconderia a correção que mais vale.
        fix_location: FixLocation::Software,
        severity: FindingSeverity::Critical,
    };

    (caros, vec![finding])
}

fn nivel(valor: f64) -> String {
    match valor as u32 {
        0 => "desligado".to_string(),
        1 => "normal".to_string(),
        2 => "alto".to_string(),
        _ => "muito alto".to_string(),
    }
}

fn legivel(chave: &str) -> &str {
    match chave {
        "MSAA" => "suavização de serrilhado (MSAA)",
        "Tessellation" => "tesselação",
        "SSAO" => "oclusão de ambiente",
        "ShadowQuality" => "qualidade das sombras",
        "ReflectionQuality" => "qualidade dos reflexos",
        outro => outro,
    }
}

/// O FiveM tem arquivo próprio no mesmo formato: mexer no do jogo errado não muda nada.
fn caminhos() -> Vec<(PathBuf, &'static str)> {
    let mut lista = Vec::new();

    if let Ok(roaming) = std::env::var("APPDATA") {
        lista.push((
            PathBuf::from(&roaming).join("CitizenFX").join("gta5_settings.xml"),
            "FiveM",
        ));
    }

    if let Ok(perfil) = std::env::var("USERPROFILE") {
        lista.push((
            PathBuf::from(&perfil)
                .join("Documents")
                .join("Rockstar Games")
                .join("GTA V")
                .join("settings.xml"),
            "GTA V",
        ));
    }

    lista
}

/// Ordem pelo custo real, não pelo menu: o MSAA primeiro em todos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Perfil {
    /// Preço visual zero, e costuma ser o maior ganho.
    SemTeto,
    Equilibrado,
    /// "Máximo de FPS" na tela desde a 2.9; o nome interno fica para o histórico continuar desfazendo.
    Competitivo,
}

pub struct Mudanca {
    pub chave: &'static str,
    pub valor: &'static str,
    pub custo: &'static str,
}

/// Um teto escrito num arquivo, não da placa: tirá-lo devolve na hora o que a máquina já entregava. Chave ausente
/// é pulada, nunca criada.
const SEM_TETO: &[Mudanca] = &[
    Mudanca { chave: "VSync", valor: "0", custo: "" },
    Mudanca { chave: "VSyncMode", valor: "0", custo: "" },
    Mudanca { chave: "FPSLimit", valor: "0", custo: "" },
    Mudanca { chave: "MaxFPS", valor: "0", custo: "" },
];

/// Chave que não existe é ignorada: o pior caso de um nome errado é não fazer nada.
const CAROS_E_POUCO_VISIVEIS: &[Mudanca] = &[
    Mudanca { chave: "MSAA", valor: "0", custo: "serrilhado nas bordas" },
    // Separada do MSAA do jogo e das mais caras: renderiza a cena de novo em cada superfície refletiva.
    Mudanca { chave: "ReflectionMSAA", valor: "0", custo: "reflexos um pouco mais serrilhados" },
    Mudanca { chave: "Tessellation", valor: "0", custo: "relevo em algumas superfícies" },
    Mudanca { chave: "SSAO", valor: "0", custo: "sombra suave nos cantos" },
    Mudanca { chave: "MotionBlurStrength", valor: "0.000000", custo: "sem desfoque de movimento" },
    Mudanca { chave: "DoF", valor: "false", custo: "fundo sempre nítido" },
    // Quanta textura carrega de uma vez: com pouca VRAM vira engasgo.
    Mudanca { chave: "HdStreamingInFlight", valor: "false", custo: "" },
];

/// Tela cheia exclusiva: o jogo fala direto com a placa, sem o compositor, imagem igual. Fora do botão automático:
/// alternar de janela fica mais lento, e quem usa Discord ou live no segundo monitor sente. `0` tela cheia, `1`
/// janela, `2` sem borda.
const TELA_CHEIA: &[Mudanca] = &[Mudanca {
    chave: "Windowed",
    valor: "0",
    custo: "alternar para outra janela fica mais lento",
}];

/// Só quando a VRAM não cabe: textura quase não custa quadro até transbordar (4 GB em RP pesado), e aí é engasgo
/// severo.
const TEXTURA_QUANDO_A_VRAM_NAO_CABE: &[Mudanca] = &[Mudanca {
    chave: "TextureQuality",
    valor: "0",
    custo: "texturas menos detalhadas de perto",
}];

const O_RESTO_QUE_CUSTA: &[Mudanca] = &[
    Mudanca { chave: "ReflectionQuality", valor: "0", custo: "reflexos mais simples" },
    Mudanca { chave: "ShadowQuality", valor: "0", custo: "sombras mais duras" },
    Mudanca { chave: "WaterQuality", valor: "0", custo: "água mais simples" },
    Mudanca { chave: "ParticleQuality", valor: "0", custo: "efeitos mais simples" },
    Mudanca { chave: "PostFX", valor: "0", custo: "menos brilho e desfoque" },
    // Das três mais caras e visível: fica no perfil que assume o custo visual.
    Mudanca { chave: "GrassQuality", valor: "0", custo: "grama mais rala e mais curta" },
    Mudanca { chave: "ShaderQuality", valor: "0", custo: "materiais menos detalhados" },
    // Aliviam o PROCESSADOR, o gargalo do FiveM cheio; qualidade gráfica não encosta nele.
    Mudanca { chave: "LodScale", valor: "0.000000", custo: "detalhe some mais perto" },
    Mudanca { chave: "PedLodBias", valor: "0.000000", custo: "pessoas distantes mais simples" },
    Mudanca { chave: "VehicleLodBias", valor: "0.000000", custo: "carros distantes mais simples" },
];

/// Depende do monitor (`display::hz_maximo()`): "180" num monitor de 60 pediria um modo que não existe. `None`
/// sem monitor legível ou já no máximo.
fn hz_do_monitor() -> Option<u32> {
    let monitores = super::display::monitores();
    let melhor = monitores.iter().map(|m| m.hz_maximo()).max()?;

    if melhor >= 60 {
        Some(melhor)
    } else {
        None
    }
}

impl Perfil {
    /// `vram_gb` decide a textura (ver `TEXTURA_QUANDO_A_VRAM_NAO_CABE`). `None` não mexe nela: custo visual sem saber
    /// se rende.
    pub fn mudancas_para(self, vram_gb: Option<f64>) -> Vec<&'static Mudanca> {
        let mut lista: Vec<&Mudanca> = SEM_TETO.iter().collect();

        if matches!(self, Perfil::Equilibrado | Perfil::Competitivo) {
            lista.extend(CAROS_E_POUCO_VISIVEIS.iter());
            lista.extend(TELA_CHEIA.iter());
        }

        if matches!(self, Perfil::Competitivo) {
            lista.extend(O_RESTO_QUE_CUSTA.iter());

            if matches!(vram_gb, Some(gb) if gb < VRAM_DE_PLACA_MODESTA_GB) {
                lista.extend(TEXTURA_QUANDO_A_VRAM_NAO_CABE.iter());
            }
        }

        lista
    }

}

fn taxa_a_corrigir(conteudo: &str) -> Option<(String, u32)> {
    let alvo = hz_do_monitor()?;
    taxa_a_corrigir_com(conteudo, alvo)
}

/// **Pura**: o teste não depende do monitor da máquina que roda.
fn taxa_a_corrigir_com(conteudo: &str, hz_do_monitor: u32) -> Option<(String, u32)> {
    let atual = valor(conteudo, "RefreshRate")?;
    let numero: u32 = atual.trim().parse().ok()?;

    // Só sobe: se o arquivo pede mais, quem pode estar errada é a nossa leitura do monitor.
    if numero >= hz_do_monitor {
        return None;
    }

    Some((atual, hz_do_monitor))
}

/// Só chaves que EXISTEM e diferem: listar as que já estão certas prometeria ganho que não vem.
pub fn prever(conteudo: &str, perfil: Perfil) -> Vec<(String, String, String, &'static str)> {
    let mut previsto = Vec::new();

    // Em TODOS os perfis: a velocidade do monitor não muda como o jogo se parece.
    if let Some((atual, alvo)) = taxa_a_corrigir(conteudo) {
        previsto.push((
            "RefreshRate".to_string(),
            atual,
            alvo.to_string(),
            "",
        ));
    }

    for m in perfil.mudancas_para(Some(vram_gb())) {
        let Some(atual) = valor(conteudo, m.chave) else {
            continue;
        };

        if atual.trim() == m.valor {
            continue;
        }

        previsto.push((
            m.chave.to_string(),
            atual,
            m.valor.to_string(),
            m.custo,
        ));
    }

    previsto
}

/// Sem mudança, conteúdo idêntico e lista vazia: quem chama não grava. A VRAM é parâmetro obrigatório: `None`
/// muda o resultado, e um atalho esconderia essa escolha. Pura.
pub fn aplicar_no_texto_com(
    conteudo: &str,
    perfil: Perfil,
    vram_gb: Option<f64>,
) -> (String, Vec<String>) {
    let mut saida = conteudo.to_string();
    let mut mexidas = Vec::new();

    if let Some((atual, alvo)) = taxa_a_corrigir(&saida) {
        if let Some(novo) = trocar(&saida, "RefreshRate", &alvo.to_string()) {
            mexidas.push(format!("RefreshRate: {} → {}", atual, alvo));
            saida = novo;
        }
    }

    for m in perfil.mudancas_para(vram_gb) {
        let Some(atual) = valor(&saida, m.chave) else {
            continue;
        };

        if atual.trim() == m.valor {
            continue;
        }

        if let Some(novo) = trocar(&saida, m.chave, m.valor) {
            mexidas.push(format!("{}: {} → {}", m.chave, atual, m.valor));
            saida = novo;
        }
    }

    (saida, mexidas)
}

pub fn analyze() -> ConfigJogoReport {
    let vram = vram_gb();

    for (caminho, jogo) in caminhos() {
        let Ok(conteudo) = std::fs::read_to_string(&caminho) else {
            continue;
        };

        let (caros, findings) = diagnosticar(&conteudo, vram);

        return ConfigJogoReport {
            arquivo: Some(caminho),
            jogo: jogo.to_string(),
            caros,
            findings,
        };
    }

    ConfigJogoReport {
        arquivo: None,
        jogo: String::new(),
        caros: Vec::new(),
        findings: Vec::new(),
    }
}

/// Pelo registro (via `bottleneck.rs`): o WMI de 32 bits satura em 4 GB.
fn vram_gb() -> f64 {
    super::bottleneck::vram_total_gb()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AplicacaoNoJogo {
    pub jogo: String,
    pub arquivo: PathBuf,
    pub mudou: Vec<String>,
    pub anterior: String,
}

/// A ÚNICA função do produto que escreve num arquivo do cliente. Recusa com o jogo aberto (ele reescreve ao
/// fechar); devolve o arquivo INTEIRO para `ChangeRecord::GameConfig`; não grava quando não há o que mudar.
pub fn aplicar_perfil(perfil: Perfil) -> Result<AplicacaoNoJogo, String> {
    let (jogo_aberto, _) = super::fivem::processos_abertos();

    if jogo_aberto {
        return Err(
            "O jogo está aberto. Feche-o antes: ele guarda a configuração em memória e \
             reescreve o arquivo ao sair, apagando o que for mudado agora."
                .to_string(),
        );
    }

    for (caminho, jogo) in caminhos() {
        let Ok(conteudo) = std::fs::read_to_string(&caminho) else {
            continue;
        };

        // A VRAM real decide se a textura entra.
        let (novo, mudou) = aplicar_no_texto_com(&conteudo, perfil, Some(vram_gb()));

        if mudou.is_empty() {
            return Ok(AplicacaoNoJogo {
                jogo: jogo.to_string(),
                arquivo: caminho,
                mudou,
                anterior: conteudo,
            });
        }

        std::fs::write(&caminho, &novo)
            .map_err(|e| format!("não consegui escrever em {}: {}", caminho.display(), e))?;

        return Ok(AplicacaoNoJogo {
            jogo: jogo.to_string(),
            arquivo: caminho,
            mudou,
            anterior: conteudo,
        });
    }

    Err("Não encontrei a configuração de nenhum jogo conhecido neste computador.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG_REAL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Settings>
  <graphics>
    <Tessellation value="1" />
    <ShadowQuality value="1" />
    <ReflectionQuality value="0" />
    <SSAO value="1" />
    <MSAA value="4" />
    <TextureQuality value="0" />
    <GrassQuality value="0" />
    <ShaderQuality value="0" />
  </graphics>
</Settings>"#;

    /// A guarda que proibia escrever passou a exigir o conteúdo anterior em toda escrita.
    #[test]
    fn toda_escrita_guarda_o_arquivo_anterior() {
        let producao = include_str!("configjogo.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();

        let escritas = producao.matches("fs::write").count();

        assert_eq!(
            escritas, 1,
            "há {} escritas neste módulo; só `aplicar_perfil` pode escrever, \
             porque só ela guarda o arquivo anterior",
            escritas
        );

        assert!(
            producao.contains("pub anterior: String"),
            "a aplicação parou de devolver o arquivo anterior; o desfazer morre com isso"
        );

        assert!(
            producao.contains("processos_abertos"),
            "a recusa de escrever com o jogo aberto sumiu"
        );
    }

    /// Um `settings.xml` completo com os valores de quem nunca mexeu em nada.
    const CONFIG_CHEIA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Settings>
  <graphics>
    <Tessellation value="2" />
    <LodScale value="1.000000" />
    <PedLodBias value="1.000000" />
    <VehicleLodBias value="1.000000" />
    <ShadowQuality value="2" />
    <ReflectionQuality value="2" />
    <ReflectionMSAA value="4" />
    <SSAO value="2" />
    <AnisotropicFiltering value="16" />
    <MSAA value="4" />
    <MotionBlurStrength value="1.000000" />
    <DoF value="true" />
    <HdStreamingInFlight value="true" />
    <TextureQuality value="2" />
    <ParticleQuality value="2" />
    <GrassQuality value="3" />
    <ShaderQuality value="2" />
    <WaterQuality value="2" />
    <PostFX value="3" />
  </graphics>
  <video>
    <VSync value="1" />
    <Windowed value="2" />
  </video>
</Settings>"#;

    /// `cargo test --lib configjogo -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn o_que_cada_perfil_muda_num_arquivo_cheio() {
        for (nome, perfil) in [
            ("SEM TETO", Perfil::SemTeto),
            ("EQUILIBRADO", Perfil::Equilibrado),
            ("COMPETITIVO", Perfil::Competitivo),
        ] {
            let (_, mexidas) = aplicar_no_texto_com(CONFIG_CHEIA, perfil, Some(4.0));

            println!("\n=== {} — {} mudança(s) ===", nome, mexidas.len());
            for m in &mexidas {
                println!("  {}", m);
            }
        }
    }

    #[test]
    fn o_competitivo_alcanca_o_que_e_caro_num_arquivo_completo() {
        let (_, mexidas) = aplicar_no_texto_com(CONFIG_CHEIA, Perfil::Competitivo, None);

        for esperado in [
            "MSAA:",
            "ReflectionMSAA:",
            "GrassQuality:",
            "ShaderQuality:",
            "LodScale:",
            "PedLodBias:",
            "VehicleLodBias:",
            "PostFX:",
            "DoF:",
            "MotionBlurStrength:",
            "HdStreamingInFlight:",
        ] {
            assert!(
                mexidas.iter().any(|m| m.starts_with(esperado)),
                "o perfil competitivo não alcançou `{}`:\n{:?}",
                esperado,
                mexidas
            );
        }
    }

    #[test]
    fn o_filtro_anisotropico_nunca_e_mexido() {
        // Quase não custa quadro e muda bastante a aparência de perto.
        for vram in [None, Some(4.0), Some(12.0)] {
            let (_, mexidas) = aplicar_no_texto_com(CONFIG_CHEIA, Perfil::Competitivo, vram);

            assert!(
                !mexidas.iter().any(|m| m.starts_with("AnisotropicFiltering:")),
                "filtro anisotrópico mexido com vram {:?}",
                vram
            );
        }
    }

    #[test]
    fn a_textura_so_cai_quando_a_memoria_de_video_nao_cabe() {
        let (_, com_placa_boa) = aplicar_no_texto_com(CONFIG_CHEIA, Perfil::Competitivo, Some(12.0));
        assert!(
            !com_placa_boa.iter().any(|m| m.starts_with("TextureQuality:")),
            "textura derrubada numa placa que tem folga de sobra"
        );

        let (_, com_placa_apertada) =
            aplicar_no_texto_com(CONFIG_CHEIA, Perfil::Competitivo, Some(4.0));
        assert!(
            com_placa_apertada.iter().any(|m| m.starts_with("TextureQuality:")),
            "a textura não caiu numa placa de 4 GB, onde ela vira engasgo"
        );

        let (_, sem_saber) = aplicar_no_texto_com(CONFIG_CHEIA, Perfil::Competitivo, None);
        assert!(
            !sem_saber.iter().any(|m| m.starts_with("TextureQuality:")),
            "a textura caiu sem o produto saber quanta memória de vídeo existe"
        );
    }

    #[test]
    fn a_tela_cheia_nao_entra_no_perfil_sem_custo() {
        // Por isso não está no `SemTeto`, o perfil do botão automático.
        assert!(
            !Perfil::SemTeto.mudancas_para(None).iter().any(|m| m.chave == "Windowed"),
            "o perfil sem custo passou a mexer no modo de tela"
        );

        assert!(
            Perfil::Equilibrado.mudancas_para(None).iter().any(|m| m.chave == "Windowed"),
            "o modo de tela sumiu do perfil do meio"
        );

        let tela = &TELA_CHEIA[0];
        assert!(!tela.custo.is_empty());
    }

    #[test]
    fn chave_que_nao_existe_no_arquivo_nao_faz_nada() {
        // O arquivo varia por versão e mod: chave ausente precisa ser não-evento.
        let (saida, mexidas) = aplicar_no_texto_com(CONFIG_REAL, Perfil::Competitivo, None);

        for inexistente in ["ReflectionMSAA", "DoF", "LodScale", "PedLodBias"] {
            assert!(
                !saida.contains(inexistente),
                "`{}` não está no arquivo e mesmo assim foi escrito",
                inexistente
            );
            assert!(
                !mexidas.iter().any(|m| m.starts_with(inexistente)),
                "`{}` foi relatado como mexido sem existir no arquivo",
                inexistente
            );
        }

        assert!(mexidas.iter().any(|m| m.starts_with("MSAA:")), "{:?}", mexidas);
    }

    #[test]
    fn a_grama_entra_no_perfil_competitivo() {
        let competitivo: Vec<&str> = Perfil::Competitivo
            .mudancas_para(None)
            .iter()
            .map(|m| m.chave)
            .collect();

        assert!(competitivo.contains(&"GrassQuality"), "{:?}", competitivo);

        let equilibrado: Vec<&str> = Perfil::Equilibrado
            .mudancas_para(None)
            .iter()
            .map(|m| m.chave)
            .collect();

        assert!(!equilibrado.contains(&"GrassQuality"));
    }

    #[test]
    fn o_equilibrado_nao_cobra_custo_visual_grande() {
        // O que muda a CARA do jogo fica para quem escolheu o competitivo.
        const SO_NO_COMPETITIVO: &[&str] = &[
            "GrassQuality",
            "ShadowQuality",
            "ReflectionQuality",
            "ShaderQuality",
            "PostFX",
        ];

        for chave in Perfil::Equilibrado.mudancas_para(None).iter().map(|m| m.chave) {
            assert!(
                !SO_NO_COMPETITIVO.contains(&chave),
                "`{}` muda a cara do jogo e não pode estar no perfil do meio",
                chave
            );
        }
    }

    #[test]
    fn os_ajustes_que_aliviam_o_processador_existem() {
        let competitivo: Vec<&str> = Perfil::Competitivo
            .mudancas_para(None)
            .iter()
            .map(|m| m.chave)
            .collect();

        for chave in ["LodScale", "PedLodBias", "VehicleLodBias"] {
            assert!(competitivo.contains(&chave), "faltou `{}`", chave);
        }
    }

    #[test]
    fn todo_ajuste_com_custo_visual_diz_qual_e() {
        for m in Perfil::Competitivo.mudancas_para(None) {
            let sem_custo_visual = matches!(m.chave, "HdStreamingInFlight")
                || SEM_TETO.iter().any(|s| s.chave == m.chave);

            if !sem_custo_visual {
                assert!(
                    !m.custo.is_empty(),
                    "`{}` muda a imagem e não diz o que o cliente perde",
                    m.chave
                );
            }
        }
    }

    #[test]
    fn trocar_mexe_so_na_chave_pedida() {
        let novo = trocar(CONFIG_REAL, "MSAA", "0").expect("MSAA existe no arquivo");

        assert_eq!(valor(&novo, "MSAA").as_deref(), Some("0"));

        assert_eq!(novo.len(), CONFIG_REAL.len());
        assert_eq!(
            valor(&novo, "Tessellation"),
            valor(CONFIG_REAL, "Tessellation")
        );
        assert_eq!(valor(&novo, "ShadowQuality"), valor(CONFIG_REAL, "ShadowQuality"));

        assert!(novo.starts_with("<?xml"));
    }

    #[test]
    fn trocar_nao_cria_chave_que_o_jogo_nao_tem() {
        assert!(trocar(CONFIG_REAL, "NaoExisteEssaChave", "0").is_none());
    }

    #[test]
    fn jogo_em_60hz_com_monitor_de_180_e_corrigido() {
        const ARQUIVO: &str = r#"<Settings>
  <video>
    <RefreshRate value="60" />
  </video>
</Settings>"#;

        let (atual, alvo) = taxa_a_corrigir_com(ARQUIVO, 180).expect("60 num monitor de 180 é trava");

        assert_eq!(atual, "60");
        assert_eq!(alvo, 180);
    }

    #[test]
    fn jogo_ja_na_taxa_do_monitor_nao_vira_mudanca() {
        const ARQUIVO: &str = r#"<RefreshRate value="180" />"#;

        assert!(taxa_a_corrigir_com(ARQUIVO, 180).is_none());
    }

    #[test]
    fn nunca_baixa_a_taxa_do_jogo() {
        const ARQUIVO: &str = r#"<RefreshRate value="240" />"#;

        assert!(taxa_a_corrigir_com(ARQUIVO, 60).is_none());
    }

    #[test]
    fn arquivo_sem_taxa_nao_recebe_a_chave() {
        assert!(taxa_a_corrigir_com("<Settings/>", 180).is_none());
    }

    #[test]
    fn sem_teto_nao_toca_em_nada_visual() {
        for m in Perfil::SemTeto.mudancas_para(None) {
            assert!(
                m.custo.is_empty(),
                "`{}` está no perfil sem custo visual e cobra `{}`",
                m.chave,
                m.custo
            );
        }

        assert!(
            !Perfil::SemTeto.mudancas_para(None).iter().any(|m| m.chave == "MSAA"),
            "o MSAA entrou no perfil que promete não mexer no visual"
        );
    }

    #[test]
    fn aplicar_no_arquivo_real_derruba_o_msaa() {
        let (novo, mudou) = aplicar_no_texto_com(CONFIG_REAL, Perfil::Equilibrado, None);

        assert_eq!(valor(&novo, "MSAA").as_deref(), Some("0"));
        assert!(
            mudou.iter().any(|m| m.starts_with("MSAA: 4 → 0")),
            "o que mudou não foi relatado: {:?}",
            mudou
        );

        assert!(
            !mudou.iter().any(|m| m.starts_with("ShaderQuality")),
            "relatou mudança numa chave que já estava no valor certo"
        );
    }

    #[test]
    fn aplicar_e_idempotente() {
        let (uma, _) = aplicar_no_texto_com(CONFIG_REAL, Perfil::Competitivo, None);
        let (duas, mudou) = aplicar_no_texto_com(&uma, Perfil::Competitivo, None);

        assert_eq!(uma, duas);
        assert!(
            mudou.is_empty(),
            "a segunda aplicação achou o que mudar: {:?}",
            mudou
        );
    }

    #[test]
    fn prever_lista_chave_valor_e_custo() {
        let previsto = prever(CONFIG_REAL, Perfil::Equilibrado);

        let msaa = previsto
            .iter()
            .find(|(chave, ..)| chave == "MSAA")
            .expect("o MSAA precisa aparecer na previsão");

        assert_eq!(msaa.1, "4", "o valor atual saiu errado");
        assert_eq!(msaa.2, "0", "o valor novo saiu errado");
        assert!(!msaa.3.is_empty(), "o cliente precisa saber o que perde");
    }

    #[test]
    fn o_msaa_da_maquina_que_motivou_o_modulo_e_apontado() {
        let (caros, findings) = diagnosticar(CONFIG_REAL, 4.0);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Critical);

        assert_eq!(caros[0].chave, "MSAA");
        assert_eq!(caros[0].valor, "4x");
        assert!(findings[0].measured.contains("MSAA) em 4x"));
    }

    #[test]
    fn o_achado_oferece_o_conserto_e_o_caminho_a_mao() {
        // O produto passou a consertar, mas o achado nunca vira reclamação sem saída e o caminho manual não some.
        let (_, findings) = diagnosticar(CONFIG_REAL, 4.0);
        let conselho = &findings[0].advice;

        assert!(
            conselho.contains("desfazer"),
            "oferecer mexer no jogo sem falar em desfazer é a metade errada da oferta"
        );
        assert!(
            conselho.contains("Gráficos → MSAA → Desligado"),
            "o caminho manual sumiu; quem prefere fazer à mão ficou sem saída"
        );
        assert_eq!(findings[0].fix_location, FixLocation::Software);
    }

    #[test]
    fn o_ganho_e_declarado_como_medicao_de_terceiro() {
        // Nunca como promessa nossa: não medimos esse número.
        let (_, findings) = diagnosticar(CONFIG_REAL, 4.0);

        assert!(findings[0].advice.contains("Análises independentes"));
    }

    #[test]
    fn placa_boa_com_a_mesma_configuracao_nao_vira_achado() {
        let (caros, findings) = diagnosticar(CONFIG_REAL, 12.0);

        assert!(caros.is_empty());
        assert!(findings.is_empty());
    }

    #[test]
    fn sem_saber_a_placa_o_modulo_fica_calado() {
        // Zero é não ter lido a VRAM, não placa fraca.
        let (caros, findings) = diagnosticar(CONFIG_REAL, 0.0);

        assert!(caros.is_empty());
        assert!(findings.is_empty());
    }

    #[test]
    fn configuracao_ja_leve_nao_vira_achado() {
        let leve = r#"<Settings><graphics>
            <MSAA value="0" />
            <Tessellation value="0" />
            <SSAO value="0" />
            <ShadowQuality value="1" />
            <ReflectionQuality value="0" />
        </graphics></Settings>"#;

        let (caros, findings) = diagnosticar(leve, 4.0);

        assert!(caros.is_empty(), "nada a apontar: {:?}", caros);
        assert!(findings.is_empty());
    }

    #[test]
    fn le_o_formato_do_jogo() {
        assert_eq!(valor(CONFIG_REAL, "MSAA").as_deref(), Some("4"));
        assert_eq!(valor(CONFIG_REAL, "TextureQuality").as_deref(), Some("0"));
        assert_eq!(valor(CONFIG_REAL, "NaoExiste"), None);
        assert_eq!(valor(CONFIG_REAL, "MSA"), None);
    }

    #[test]
    fn le_a_configuracao_desta_maquina() {
        let r = analyze();

        match &r.arquivo {
            Some(caminho) => {
                println!("  {} → {}", r.jogo, caminho.display());
                for c in &r.caros {
                    println!("    {} em {} · custa {} · {}", c.chave, c.valor, c.ganho, c.onde);
                }
                for f in &r.findings {
                    println!("  [{:?}] {}", f.severity, f.measured);
                }
            }
            None => println!("  nenhum jogo com configuração legível nesta máquina"),
        }

        if let Some(caminho) = &r.arquivo {
            assert!(caminho.exists());
        }
    }
}
