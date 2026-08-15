// A configuração do jogo — lida, nunca escrita
//
// POR QUE ESTE MÓDULO EXISTE
//
// O dono aplicou todas as otimizações do produto e disse que o FiveM continuou
// igual. Estava certo, e o motivo é desconfortável: o FPS dele não estava onde
// o produto — nem nenhum concorrente — estava procurando.
//
// A configuração gráfica da máquina dele, medida:
//
//     TextureQuality 0 · GrassQuality 0 · ShaderQuality 0 · WaterQuality 0
//     ParticleQuality 0 · PostFX 0 · ReflectionQuality 0 · CityDensity 0.0
//     MSAA 4          <- quatro vezes
//
// Tudo no mínimo, e o MSAA em 4x. Numa placa de entrada a 1080p, o MSAA é a
// configuração mais cara que existe no GTA V: análises independentes medem
// entre 30% e 50% dos quadros só nele. Tudo o que ele desligou junto vale
// menos que essa única linha.
//
// A hierarquia real, para quem for mexer aqui depois:
//
//     uma configuração de jogo mal escolhida ... dezenas de por cento
//     memória insuficiente ..................... o teto da máquina
//     ajustes de Windows, todos somados ........ alguns por cento
//
// ESTE MÓDULO NÃO ESCREVE. NUNCA.
//
// A regra é do dono do produto: o Otimiza mexe no PC, não no jogo. Mexer na
// configuração de um jogo é mexer em como ele se parece, e essa escolha é de
// quem joga.
//
// Mas ficar CALADO sobre 40% de FPS por causa de uma regra de escopo seria o
// produto escondendo do cliente a coisa mais valiosa que ele sabe. Então aqui
// se lê, se explica, e se diz onde clicar — igual o produto já faz com a taxa
// de atualização do monitor.
//
// A ausência de escrita é garantida por teste (`este_modulo_nunca_escreve`),
// no mesmo espírito da guarda que impede a suspensão de virar "matar processo".

use super::achados::{FindingSeverity, FixLocation};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Uma configuração que custa caro e o que ela custa.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AjusteCaro {
    pub chave: String,
    pub valor: String,
    /// O que fazer, em palavras do menu do jogo.
    pub onde: String,
    /// Faixa de ganho, medida por terceiros — nunca por nós.
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
    /// Onde o arquivo foi encontrado. Vazio quando o jogo não está instalado.
    pub arquivo: Option<PathBuf>,
    pub jogo: String,
    /// O que está pesando, do mais caro para o menos.
    pub caros: Vec<AjusteCaro>,
    pub findings: Vec<ConfigJogoFinding>,
}

/// Lê `<Nome value="X" />` do formato do GTA V.
///
/// Analisador de uma linha em vez de biblioteca de XML: o arquivo tem um
/// formato rígido gerado pelo próprio jogo, e trazer uma dependência nova para
/// o instalador por causa disto não se paga. Como o módulo só LÊ, um engano
/// aqui produz um diagnóstico errado — nunca um arquivo corrompido.
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

/// Placas com esta memória de vídeo ou menos não têm folga para gastar com
/// suavização de serrilhado.
///
/// Não é sobre a memória em si: é o critério mais simples e confiável para
/// separar placa de entrada de placa que aguenta. Uma GTX 1650 tem 4 GB.
const VRAM_DE_PLACA_MODESTA_GB: f64 = 6.0;

/// Decide o que está caro para ESTA máquina.
///
/// **Função pura.** A mesma configuração é problema numa placa de entrada e não
/// é problema nenhuma numa placa boa — e apontar MSAA 4x para quem tem uma
/// RTX 4070 seria inventar problema para parecer útil.
pub fn diagnosticar(conteudo: &str, vram_gb: f64) -> (Vec<AjusteCaro>, Vec<ConfigJogoFinding>) {
    let mut caros = Vec::new();
    let placa_modesta = vram_gb > 0.0 && vram_gb <= VRAM_DE_PLACA_MODESTA_GB;

    // MSAA é o primeiro da lista por uma razão: ele sozinho custa mais que
    // todos os outros juntos. E é o mais fácil de deixar ligado sem perceber,
    // porque quem baixa as configurações no menu mexe nas linhas de "qualidade"
    // e passa direto por esta.
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
             sistema, somado. O Otimiza NÃO mexe na configuração do seu jogo: quem \
             decide como o jogo se parece é você. Para mudar, no menu do próprio \
             jogo: {}.",
            passos.join("; ")
        ),
        // Não é `Software`: o Otimiza não conserta isto, e marcar como se
        // consertasse faria a interface oferecer um botão que não existe.
        fix_location: FixLocation::None,
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

// ------------------------------------------------------------------- leitura

/// Onde o arquivo de configuração pode estar.
///
/// O FiveM usa o mesmo formato do GTA V, com arquivo próprio: quem joga RP tem
/// os dois, com configurações diferentes, e mexer no do jogo errado não muda
/// nada — motivo pelo qual tanta gente "baixa tudo" e não vê diferença.
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

/// Memória da placa de vídeo, em GB.
///
/// Reaproveita a leitura que `bottleneck.rs` já faz do registro: o valor de 32
/// bits do WMI satura em 4 GB e mentiria justamente na faixa que interessa.
fn vram_gb() -> f64 {
    super::bottleneck::vram_total_gb()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Os valores reais da máquina que motivou este módulo.
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

    #[test]
    fn este_modulo_nunca_escreve() {
        // A regra do dono do produto: o Otimiza mexe no PC, não no jogo. Como o
        // arquivo lido aqui é do cliente e descreve a experiência dele, uma
        // escrita acidental seria mudar como o jogo dele se parece sem ele
        // pedir. O teste tranca isso no nível do arquivo.
        let producao = include_str!("configjogo.rs").split("#[cfg(test)]").next().unwrap();

        for proibido in [
            "fs::write",
            "fs::remove",
            "File::create",
            "OpenOptions",
            "write_all",
            "set_dword",
            "set_string",
        ] {
            assert!(
                !producao.contains(proibido),
                "`{}` apareceu num módulo que só pode ler",
                proibido
            );
        }
    }

    #[test]
    fn o_msaa_da_maquina_que_motivou_o_modulo_e_apontado() {
        // GTX 1650, 4 GB. Tudo no mínimo e o MSAA em 4x.
        let (caros, findings) = diagnosticar(CONFIG_REAL, 4.0);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Critical);

        // O MSAA vem primeiro: sozinho ele custa mais que todo o resto junto.
        assert_eq!(caros[0].chave, "MSAA");
        assert_eq!(caros[0].valor, "4x");
        assert!(findings[0].measured.contains("MSAA) em 4x"));
    }

    #[test]
    fn o_produto_diz_que_nao_vai_mexer_e_onde_clicar() {
        // Sem isto, o achado vira uma reclamação sem saída.
        let (_, findings) = diagnosticar(CONFIG_REAL, 4.0);
        let conselho = &findings[0].advice;

        assert!(conselho.contains("NÃO mexe na configuração do seu jogo"));
        assert!(conselho.contains("Gráficos → MSAA → Desligado"));
        // E o achado não pode fingir que o Otimiza conserta.
        assert_eq!(findings[0].fix_location, FixLocation::None);
    }

    #[test]
    fn o_ganho_e_declarado_como_medicao_de_terceiro() {
        // Nunca como promessa nossa: não medimos esse número, e afirmar que
        // medimos seria exatamente o que este produto não faz.
        let (_, findings) = diagnosticar(CONFIG_REAL, 4.0);

        assert!(findings[0].advice.contains("Análises independentes"));
    }

    #[test]
    fn placa_boa_com_a_mesma_configuracao_nao_vira_achado() {
        // A mesma configuração numa placa que aguenta não é problema, e apontar
        // seria inventar defeito para parecer útil.
        let (caros, findings) = diagnosticar(CONFIG_REAL, 12.0);

        assert!(caros.is_empty());
        assert!(findings.is_empty());
    }

    #[test]
    fn sem_saber_a_placa_o_modulo_fica_calado() {
        // Zero significa "não conseguimos ler a memória de vídeo", e não
        // "placa fraca". Afirmar com base em ausência de dado é inventar.
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
        // Chave parecida não pode casar com outra.
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

        // O arquivo relatado precisa existir de verdade.
        if let Some(caminho) = &r.arquivo {
            assert!(caminho.exists());
        }
    }
}
