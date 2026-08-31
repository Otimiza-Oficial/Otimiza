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
// ESTE MÓDULO PASSOU A ESCREVER, E A REGRA QUE O PROIBIA ERA DO DONO.
//
// Ela dizia: "o Otimiza mexe no PC, não no jogo". Fazia sentido — mexer na
// configuração de um jogo é mexer em como ele se parece, e essa escolha é de
// quem joga. O dono a removeu, e removeu certo: ficar calado sobre dezenas de
// por cento de FPS por causa de uma regra de escopo era o produto escondendo do
// cliente a coisa mais valiosa que ele sabe.
//
// O RISCO QUE A REGRA COBRIA NÃO DESAPARECEU COM ELA.
//
// Escrever no arquivo do cliente sem caminho de volta continua sendo a pior
// coisa que este módulo pode fazer. Então três coisas seguram isso, e nenhuma é
// opcional:
//
//   1. Uma escrita só, em `aplicar_perfil`, e ela devolve o arquivo INTEIRO
//      como estava — é o que o desfazer reescreve.
//   2. Recusa de escrever com o jogo aberto: ele guarda a configuração em
//      memória e reescreve ao sair, apagando o que fizermos agora.
//   3. `trocar()` mexe só nos caracteres entre as aspas daquela chave. O resto
//      do arquivo — comentários, ordem, indentação — sai byte a byte igual.
//
// A guarda antiga não foi apagada: virou `toda_escrita_guarda_o_arquivo_anterior`,
// que exige as três. Apagar a guarda junto com a regra teria sido perder a lição
// junto com a restrição.
//
// A TERCEIRA TRAVA, ENCONTRADA NA MÁQUINA DO DONO
//
// Além das configurações caras, o arquivo dele tinha `RefreshRate = 60` num
// monitor de 180 Hz — o jogo pedindo ao Windows um terço da velocidade que a
// tela entrega. Isso não muda nada de como o jogo se parece, e por isso entra
// até no perfil que promete não mexer no visual.

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

/// Troca o valor de UMA chave, sem tocar em mais nada do arquivo.
///
/// **Função pura**: recebe o texto e devolve o texto. Não abre arquivo, não
/// decide nada. É o que permite provar a substituição com o arquivo real do
/// dono, num teste, sem nada em disco.
///
/// POR QUE SUBSTITUIÇÃO CIRÚRGICA, E NÃO REESCREVER O ARQUIVO
///
/// A tentação é ler tudo, montar uma estrutura, e gravar de volta. Isso perde o
/// que não foi entendido: comentários, chaves de versões futuras, a ordem das
/// linhas, a indentação, o cabeçalho XML. O jogo escreveu aquele arquivo do
/// jeito dele, e devolver "equivalente" é como devolver um texto reescrito com
/// as mesmas palavras — pode funcionar, e não é a mesma coisa.
///
/// Aqui só os caracteres entre as aspas daquela chave mudam. Todo o resto do
/// arquivo sai byte a byte igual ao que entrou.
///
/// Devolve `None` quando a chave não existe: nesse caso não há o que trocar, e
/// inventar a linha seria escrever no arquivo do jogo uma configuração que o
/// jogo talvez nem leia naquela versão.
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
             sistema, somado. O Otimiza consegue mudar isto para você, mostrando \
             antes o que muda e guardando o arquivo para desfazer a qualquer \
             momento. Se preferir fazer à mão, no menu do próprio jogo: {}.",
            passos.join("; ")
        ),
        // MUDOU DE `None` PARA `Software`, e a mudança é o Pilar 1 inteiro.
        //
        // Enquanto o produto não escrevia no jogo, marcar como `Software` faria
        // a interface oferecer um botão que não existia. Agora o botão existe —
        // e manter `None` faria o contrário: esconder do cliente a correção que
        // mais vale, justamente a que ele veio buscar.
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

/// O que cada perfil faz com o arquivo do jogo.
///
/// A ORDEM DENTRO DE CADA PERFIL SEGUE O CUSTO REAL, e não o menu do jogo. O
/// MSAA vem primeiro em todos porque sozinho ele custa mais que todos os outros
/// somados numa placa de entrada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Perfil {
    /// Só tira os tetos. Não mexe em nada que mude como o jogo se parece.
    ///
    /// É o perfil que deveria ser o padrão da conversa com o cliente: o ganho
    /// costuma ser o maior de todos, e o preço visual é exatamente zero.
    SemTeto,
    /// Tira os tetos e desliga o que é caro e pouco visível.
    Equilibrado,
    /// Tira os tetos e derruba tudo que custa quadro.
    Competitivo,
}

/// Uma chave e o valor que o perfil quer nela.
pub struct Mudanca {
    pub chave: &'static str,
    pub valor: &'static str,
    /// O que o cliente perde, em português. Vazio quando não perde nada.
    pub custo: &'static str,
}

/// AS DUAS CHAVES QUE FAZEM 200 VIRAR 500.
///
/// Um jogo travado em 200 quadros não está travado pela placa: está travado por
/// um número escrito num arquivo. Tirar o teto devolve o que a máquina já era
/// capaz de fazer, e devolve na hora.
///
/// É a diferença entre as duas coisas que este produto faz: as outras
/// otimizações tentam melhorar o que a máquina entrega; esta remove uma trava
/// que estava escondendo o que ela já entregava.
///
/// `VSyncMode` e `FPSLimit` só existem em parte das versões e configurações. A
/// substituição devolve `None` quando a chave não está no arquivo, e o perfil
/// simplesmente segue para a próxima — nunca cria a linha.
const SEM_TETO: &[Mudanca] = &[
    Mudanca { chave: "VSync", valor: "0", custo: "" },
    Mudanca { chave: "VSyncMode", valor: "0", custo: "" },
    Mudanca { chave: "FPSLimit", valor: "0", custo: "" },
    Mudanca { chave: "MaxFPS", valor: "0", custo: "" },
];

const CAROS_E_POUCO_VISIVEIS: &[Mudanca] = &[
    Mudanca { chave: "MSAA", valor: "0", custo: "serrilhado nas bordas" },
    Mudanca { chave: "Tessellation", valor: "0", custo: "relevo em algumas superfícies" },
    Mudanca { chave: "SSAO", valor: "0", custo: "sombra suave nos cantos" },
];

const O_RESTO_QUE_CUSTA: &[Mudanca] = &[
    Mudanca { chave: "ReflectionQuality", valor: "0", custo: "reflexos mais simples" },
    Mudanca { chave: "ShadowQuality", valor: "0", custo: "sombras mais duras" },
    Mudanca { chave: "WaterQuality", valor: "0", custo: "água mais simples" },
    Mudanca { chave: "ParticleQuality", valor: "0", custo: "efeitos mais simples" },
    Mudanca { chave: "PostFX", valor: "0", custo: "menos brilho e desfoque" },
];

/// A taxa de atualização que o jogo vai pedir, lida do monitor de verdade.
///
/// ESTA É A TERCEIRA TRAVA, E FOI ENCONTRADA NA MÁQUINA DO DONO.
///
/// O arquivo dele tinha `RefreshRate = 60` num monitor de 180 Hz. Não é uma
/// configuração de qualidade: é o jogo avisando ao Windows a que velocidade
/// quer a tela. Em tela cheia exclusiva, esse número é um teto — o jogo não
/// passa dali por mais placa que exista.
///
/// Não entra na tabela fixa como as outras porque o valor certo depende do
/// monitor: escrever "180" num monitor de 60 Hz pediria um modo que não existe,
/// e o jogo cairia para algum padrão. O valor sai de `display::hz_maximo()`,
/// que é o mesmo que o produto já usa para a taxa do Windows.
///
/// Devolve `None` quando não há monitor legível ou quando ele já está no
/// máximo — e nesse caso não há trava para tirar.
fn hz_do_monitor() -> Option<u32> {
    let monitores = super::display::monitores();
    let melhor = monitores.iter().map(|m| m.hz_maximo()).max()?;

    // Abaixo disto não vale mexer: a diferença não se sente e a mudança pediria
    // um modo de vídeo por um ganho que ninguém percebe.
    if melhor >= 60 {
        Some(melhor)
    } else {
        None
    }
}

impl Perfil {
    pub fn mudancas(self) -> Vec<&'static Mudanca> {
        let mut lista: Vec<&Mudanca> = SEM_TETO.iter().collect();

        if matches!(self, Perfil::Equilibrado | Perfil::Competitivo) {
            lista.extend(CAROS_E_POUCO_VISIVEIS.iter());
        }

        if matches!(self, Perfil::Competitivo) {
            lista.extend(O_RESTO_QUE_CUSTA.iter());
        }

        lista
    }
}

/// A taxa que está no arquivo e a que o monitor aguenta, quando diferem.
///
/// Devolve `None` quando o arquivo não tem a chave, quando não dá para ler o
/// monitor, ou quando já estão iguais.
fn taxa_a_corrigir(conteudo: &str) -> Option<(String, u32)> {
    let alvo = hz_do_monitor()?;
    taxa_a_corrigir_com(conteudo, alvo)
}

/// A decisão, separada da leitura do monitor.
///
/// **Função pura**, e é ela que os testes usam: a máquina que roda o teste tem
/// o monitor que tem, e um teste que dependesse disso passaria aqui e falharia
/// na esteira.
fn taxa_a_corrigir_com(conteudo: &str, hz_do_monitor: u32) -> Option<(String, u32)> {
    let atual = valor(conteudo, "RefreshRate")?;
    let numero: u32 = atual.trim().parse().ok()?;

    // Só sobe. Se o arquivo pede MAIS do que o monitor faz, quem está errado
    // pode ser a nossa leitura do monitor — e baixar seria estragar uma
    // configuração que talvez esteja certa.
    if numero >= hz_do_monitor {
        return None;
    }

    Some((atual, hz_do_monitor))
}

/// O que um perfil FARIA neste arquivo, sem escrever nada.
///
/// Existe para a tela poder mostrar exatamente o que vai mudar antes de o
/// cliente decidir — e para a decisão dele ser sobre chaves com nome e valor,
/// não sobre a palavra "otimizar".
///
/// Só entram as chaves que EXISTEM no arquivo e cujo valor é diferente do que o
/// perfil quer. Uma chave que já está no valor certo não é mudança, e listá-la
/// faria a tela prometer um ganho que não vai acontecer.
pub fn prever(conteudo: &str, perfil: Perfil) -> Vec<(String, String, String, &'static str)> {
    let mut previsto = Vec::new();

    // A taxa de atualização entra em TODOS os perfis, inclusive no que promete
    // não mexer no visual: pedir ao jogo a velocidade real do monitor não muda
    // nada de como ele se parece.
    if let Some((atual, alvo)) = taxa_a_corrigir(conteudo) {
        previsto.push((
            "RefreshRate".to_string(),
            atual,
            alvo.to_string(),
            "",
        ));
    }

    for m in perfil.mudancas() {
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

/// Aplica o perfil ao TEXTO. Continua sem tocar em disco.
///
/// Devolve o conteúdo novo e a lista do que mudou. Quando nada muda, devolve o
/// conteúdo idêntico e uma lista vazia — e quem chama precisa tratar isso como
/// "não havia o que fazer", em vez de gravar um arquivo igual e registrar uma
/// mudança que não houve.
pub fn aplicar_no_texto(conteudo: &str, perfil: Perfil) -> (String, Vec<String>) {
    let mut saida = conteudo.to_string();
    let mut mexidas = Vec::new();

    if let Some((atual, alvo)) = taxa_a_corrigir(&saida) {
        if let Some(novo) = trocar(&saida, "RefreshRate", &alvo.to_string()) {
            mexidas.push(format!("RefreshRate: {} → {}", atual, alvo));
            saida = novo;
        }
    }

    for m in perfil.mudancas() {
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

/// Memória da placa de vídeo, em GB.
///
/// Reaproveita a leitura que `bottleneck.rs` já faz do registro: o valor de 32
/// bits do WMI satura em 4 GB e mentiria justamente na faixa que interessa.
fn vram_gb() -> f64 {
    super::bottleneck::vram_total_gb()
}

/// O resultado de aplicar um perfil no jogo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AplicacaoNoJogo {
    pub jogo: String,
    pub arquivo: PathBuf,
    /// Cada linha no formato "MSAA: 4 → 0". Vazio quando não havia o que mudar.
    pub mudou: Vec<String>,
    /// O arquivo inteiro como estava, para o registro de desfazer.
    pub anterior: String,
}

/// Escreve o perfil no arquivo do jogo.
///
/// ESTA É A ÚNICA FUNÇÃO DO PRODUTO QUE ESCREVE NUM ARQUIVO DO CLIENTE.
///
/// Todas as outras otimizações mexem no Windows — registro, serviço, plano de
/// energia — que é território do sistema. Este arquivo é do jogo, e mexer nele
/// muda como o jogo se parece para quem joga. Por isso as três garantias
/// abaixo, e nenhuma delas é opcional.
///
/// 1. RECUSA COM O JOGO ABERTO. O GTA V e o FiveM mantêm a configuração em
///    memória e reescrevem o arquivo ao fechar. Escrever com ele aberto é
///    trabalho que o jogo apaga ao sair — e o cliente veria "aplicado" seguido
///    de nada acontecendo, que é pior que não aplicar.
///
/// 2. DEVOLVE O ARQUIVO INTEIRO como estava, para o `ChangeRecord::GameConfig`.
///    O desfazer não recompõe: ele reescreve o que estava lá.
///
/// 3. NÃO ESCREVE QUANDO NÃO HÁ O QUE MUDAR. Gravar um arquivo idêntico
///    registraria uma mudança que não houve, e o cliente teria um item no
///    histórico de desfazer que não desfaz nada.
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

        let (novo, mudou) = aplicar_no_texto(&conteudo, perfil);

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

    /// A guarda que proibia escrever virou a guarda que exige guardar antes.
    ///
    /// A regra antiga era do dono: "o Otimiza mexe no PC, não no jogo". Ele a
    /// removeu, e removeu por um bom motivo — a configuração do jogo vale
    /// dezenas de por cento, enquanto os quarenta e dois ajustes de Windows
    /// somados valem alguns.
    ///
    /// Mas o RISCO que aquela regra cobria não desapareceu junto: escrever no
    /// arquivo do cliente sem ter como voltar continua sendo a pior coisa que
    /// este módulo pode fazer. Então a guarda não foi apagada, foi virada do
    /// avesso: em vez de proibir a escrita, ela exige que toda escrita venha
    /// acompanhada do conteúdo anterior.
    ///
    /// Apagar a guarda junto com a regra teria sido perder a lição junto com a
    /// restrição.
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

        // O conteúdo anterior tem que sair da função, senão não há desfazer.
        assert!(
            producao.contains("pub anterior: String"),
            "a aplicação parou de devolver o arquivo anterior; o desfazer morre com isso"
        );

        // E a recusa com o jogo aberto não pode sumir numa refatoração: sem
        // ela, o jogo apaga o trabalho ao fechar e o cliente vê "aplicado"
        // seguido de nada.
        assert!(
            producao.contains("processos_abertos"),
            "a recusa de escrever com o jogo aberto sumiu"
        );
    }

    /// Trocar uma chave não pode mexer em mais nada do arquivo.
    #[test]
    fn trocar_mexe_so_na_chave_pedida() {
        let novo = trocar(CONFIG_REAL, "MSAA", "0").expect("MSAA existe no arquivo");

        assert_eq!(valor(&novo, "MSAA").as_deref(), Some("0"));

        // Todo o resto sai idêntico: mesmo tamanho de diferença que a troca de
        // "4" por "0" justifica, e nenhuma outra chave mexida.
        assert_eq!(novo.len(), CONFIG_REAL.len());
        assert_eq!(
            valor(&novo, "Tessellation"),
            valor(CONFIG_REAL, "Tessellation")
        );
        assert_eq!(valor(&novo, "ShadowQuality"), valor(CONFIG_REAL, "ShadowQuality"));

        // O cabeçalho e os comentários continuam onde estavam.
        assert!(novo.starts_with("<?xml"));
    }

    /// Chave que não existe no arquivo não é inventada.
    #[test]
    fn trocar_nao_cria_chave_que_o_jogo_nao_tem() {
        assert!(trocar(CONFIG_REAL, "NaoExisteEssaChave", "0").is_none());
    }

    /// A trava encontrada na máquina do dono: jogo em 60 Hz, monitor em 180.
    ///
    /// Não é configuração de qualidade — é o jogo avisando ao Windows a que
    /// velocidade quer a tela. Em tela cheia exclusiva vira teto: o jogo não
    /// passa dali por mais placa que exista.
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

    /// Já no máximo: não há trava para tirar, e o produto não inventa mudança.
    #[test]
    fn jogo_ja_na_taxa_do_monitor_nao_vira_mudanca() {
        const ARQUIVO: &str = r#"<RefreshRate value="180" />"#;

        assert!(taxa_a_corrigir_com(ARQUIVO, 180).is_none());
    }

    /// O arquivo pede MAIS do que lemos do monitor: não mexemos.
    ///
    /// Quem pode estar errado nesse caso é a NOSSA leitura do monitor, e baixar
    /// a taxa estragaria uma configuração que talvez esteja certa. Só subimos.
    #[test]
    fn nunca_baixa_a_taxa_do_jogo() {
        const ARQUIVO: &str = r#"<RefreshRate value="240" />"#;

        assert!(taxa_a_corrigir_com(ARQUIVO, 60).is_none());
    }

    /// Arquivo sem a chave não ganha uma linha nova.
    #[test]
    fn arquivo_sem_taxa_nao_recebe_a_chave() {
        assert!(taxa_a_corrigir_com("<Settings/>", 180).is_none());
    }

    /// O perfil que não mexe no visual só tira teto.
    #[test]
    fn sem_teto_nao_toca_em_nada_visual() {
        for m in Perfil::SemTeto.mudancas() {
            assert!(
                m.custo.is_empty(),
                "`{}` está no perfil sem custo visual e cobra `{}`",
                m.chave,
                m.custo
            );
        }

        // E o MSAA, que é o mais caro de todos, NÃO entra nele: quem escolhe
        // "não quero mudar como o jogo se parece" não pode perder serrilhado.
        assert!(
            !Perfil::SemTeto.mudancas().iter().any(|m| m.chave == "MSAA"),
            "o MSAA entrou no perfil que promete não mexer no visual"
        );
    }

    /// Aplicar no arquivo real do dono: o MSAA 4x cai, e nada mais se perde.
    #[test]
    fn aplicar_no_arquivo_real_derruba_o_msaa() {
        let (novo, mudou) = aplicar_no_texto(CONFIG_REAL, Perfil::Equilibrado);

        assert_eq!(valor(&novo, "MSAA").as_deref(), Some("0"));
        assert!(
            mudou.iter().any(|m| m.starts_with("MSAA: 4 → 0")),
            "o que mudou não foi relatado: {:?}",
            mudou
        );

        // As chaves que já estavam no mínimo não entram na lista: contá-las
        // faria a tela prometer um ganho que não vai acontecer.
        assert!(
            !mudou.iter().any(|m| m.starts_with("ShaderQuality")),
            "relatou mudança numa chave que já estava no valor certo"
        );
    }

    /// Aplicar duas vezes seguidas não muda nada na segunda.
    #[test]
    fn aplicar_e_idempotente() {
        let (uma, _) = aplicar_no_texto(CONFIG_REAL, Perfil::Competitivo);
        let (duas, mudou) = aplicar_no_texto(&uma, Perfil::Competitivo);

        assert_eq!(uma, duas);
        assert!(
            mudou.is_empty(),
            "a segunda aplicação achou o que mudar: {:?}",
            mudou
        );
    }

    /// A previsão mostra ao cliente exatamente o que vai mudar, antes.
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
    fn o_achado_oferece_o_conserto_e_o_caminho_a_mao() {
        // Este teste mudou junto com o produto. Ele afirmava que o conselho
        // dizia "o Otimiza NÃO mexe na configuração do seu jogo" e que o achado
        // não podia fingir que o produto consertava.
        //
        // O produto passou a consertar. Manter a afirmação antiga esconderia do
        // cliente a correção que mais vale — mas as duas outras metades
        // continuam valendo, e continuam trancadas aqui: o achado nunca pode
        // virar reclamação sem saída, e o caminho manual não pode sumir só
        // porque agora existe um botão.
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
