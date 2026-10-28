// TURBO (3.5). Um botão: mede a máquina e o jogo, escolhe a receita pelo GARGALO, aplica e lembra por jogo. As peças já
// existiam soltas (Frame Boost, escala, energia de jogo, memória para o jogo); o Turbo só decide qual delas serve:
//
//   placa no limite      → escala (FPS real a mais) e, se ainda faltar para o monitor, geração
//   processador no limite → geração e energia de jogo (a escala não ajuda: a placa já sobra)
//   memória acabando     → a memória primeiro (oferecer fechar; nunca fecha sozinho) e a energia
//   nada no limite       → não liga nada e diz que o jogo já está no limite desta máquina
//
// "Menor atraso" (escolha da pessoa no cartão; jogo de tiro já vem nela) não ganha geração: ela acrescenta atraso.
// O nível da escala sai do FPS medido, como ESTIMATIVA até a medida do depois.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub use crate::modules::provaalternada::Desfecho;
use crate::modules::provaalternada::ORDEM;
use crate::modules::windows::bottleneck::Limite;
use crate::modules::windows::geracao::escala::Nivel;

/// Onde o jogo trava, nas palavras do Turbo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gargalo {
    Placa,
    Processador,
    Memoria,
    /// Nada no limite: o jogo já roda o que esta máquina consegue.
    Nenhum,
    /// O jogo estava parado (menu, pausa) durante a medida.
    SemCarga,
}

/// Pura. VRAM cheia conta como placa (menos pixels também é menos memória de vídeo); disco e "não identificado" não
/// têm receita no Turbo.
pub fn gargalo_de(l: Limite) -> Gargalo {
    match l {
        Limite::Gpu | Limite::MemoriaVideo => Gargalo::Placa,
        Limite::CpuUmNucleo | Limite::CpuTodos => Gargalo::Processador,
        Limite::MemoriaRam => Gargalo::Memoria,
        Limite::SemCarga => Gargalo::SemCarga,
        Limite::Disco | Limite::NaoIdentificado => Gargalo::Nenhum,
    }
}

/// Jogos de tiro e de reflexo: só o ponto de partida da escolha "menor atraso", que a pessoa troca no cartão. Quem
/// decide se a geração entra é ela, não a lista.
const COMPETITIVOS: &[&str] = &["cs2", "valorant", "fortnite", "cod", "apex", "rocketleague"];

pub fn competitivo(id_do_jogo: &str) -> bool {
    COMPETITIVOS.contains(&id_do_jogo)
}

/// Pura: a escolha guardada para o jogo, ou a sugestão (a lista acima) quando a pessoa ainda não escolheu.
fn menor_atraso_em(p: &Perfis, id_do_jogo: &str) -> bool {
    p.menor_atraso.get(id_do_jogo).copied().unwrap_or_else(|| competitivo(id_do_jogo))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Receita {
    pub gargalo: Gargalo,
    /// O Frame Boost: escala (nível) e multiplicador (1 = só escala, 0 = Auto, 2+ = fixo). `None` = não liga.
    pub escala: Option<Nivel>,
    pub multiplicador: Option<u8>,
    pub energia: bool,
    /// Oferecer fechar o que ocupa memória (a tela lista; o Turbo nunca fecha sozinho).
    pub memoria: bool,
    /// O que o Turbo vai fazer e por quê, para a tela.
    pub explicacao: String,
}

/// Pura: as duas receitas ligam a mesma coisa (escala, multiplicador e energia), mesmo com textos e FPS diferentes.
/// É o que impede o "Medir de novo" de religar uma receita que a prova reprovou.
pub fn mesma_acao(a: &Receita, b: &Receita) -> bool {
    a.escala == b.escala && a.multiplicador == b.multiplicador && a.energia == b.energia
}

/// Pura: o FPS do jogo renderizando no nível, preso na placa. O FPS cresce quase na proporção dos pixels, mas não
/// inteiro (geometria, processador e pós-processamento não diminuem): expoente 0,7, para não prometer demais.
pub fn fps_no_nivel(fps: f64, nivel: Nivel) -> f64 {
    fps * nivel.fator().powi(2).powf(0.7)
}

/// Pura: o nível mais bonito que alcança `alvo` (FPS do jogo). Sem FPS medido, Qualidade; nenhum alcança, Desempenho.
pub fn nivel_automatico(fps: Option<f64>, alvo: f64) -> Nivel {
    let Some(fps) = fps.filter(|f| *f > 0.0) else {
        return Nivel::Qualidade;
    };
    Nivel::TODOS
        .iter()
        .copied()
        .find(|n| fps_no_nivel(fps, *n) >= alvo)
        .unwrap_or(Nivel::Desempenho)
}

/// Pura: a receita para o gargalo medido. `fps` é o do jogo na medida (sem ele, o nível padrão); `hz` o do monitor.
pub fn montar(gargalo: Gargalo, fps: Option<f64>, hz_lido: Option<u32>, competitivo: bool) -> Receita {
    let hz = hz_lido.filter(|h| *h > 0).unwrap_or(60) as f64;
    let (escala, multiplicador, energia, memoria, explicacao) = match gargalo {
        Gargalo::Placa => {
            // Só escala tem de encher o monitor sozinha; com geração, metade dele já vira o monitor cheio no 2×.
            let so_escala = nivel_automatico(fps, hz);
            let precisa_gerar = !competitivo && fps.is_some_and(|f| fps_no_nivel(f, so_escala) < hz * 0.9);
            if precisa_gerar {
                let nivel = nivel_automatico(fps, hz / 2.0);
                (
                    Some(nivel),
                    Some(0),
                    true,
                    false,
                    "A placa de vídeo está no limite: o jogo passa a renderizar menos pixels (FPS real a mais) e, pela \
                     estimativa, a geração de quadros completa o monitor. O número medido aparece ao ligar."
                        .to_string(),
                )
            } else {
                (
                    Some(so_escala),
                    Some(1),
                    true,
                    false,
                    if competitivo {
                        "A placa de vídeo está no limite: o jogo passa a renderizar menos pixels. Menor atraso \
                         escolhido: sem geração de quadros, para não acrescentar atraso."
                    } else {
                        "A placa de vídeo está no limite: o jogo passa a renderizar menos pixels, e pela estimativa \
                         isso já basta para o monitor. Sem geração, sem atraso a mais. O número medido aparece ao \
                         ligar."
                    }
                    .to_string(),
                )
            }
        }
        Gargalo::Processador => (
            None,
            (!competitivo).then_some(0),
            true,
            false,
            if competitivo {
                "O processador está no limite: diminuir a resolução não ajudaria (a placa já sobra). O processador \
                 passa a subir de frequência sem hesitar com o jogo aberto. Menor atraso escolhido: sem geração."
            } else {
                "O processador está no limite: diminuir a resolução não ajudaria (a placa já sobra). A geração de \
                 quadros enche o monitor e o processador passa a subir de frequência sem hesitar."
            }
            .to_string(),
        ),
        Gargalo::Memoria => (
            None,
            None,
            true,
            true,
            "A memória está acabando: é ela que trava o jogo ao carregar cenário. Feche o que não precisa (a lista \
             abaixo) e meça de novo."
                .to_string(),
        ),
        // Nada no limite e o monitor sobrando: o que segura o FPS é o próprio jogo (limite, servidor, motor). Menos pixels
        // ou processador livre não ajudam; a geração enche o monitor, e a explicação diz que é FPS na tela.
        Gargalo::Nenhum if !competitivo && fps.is_some_and(|f| f < hz * 0.9) => (
            None,
            Some(0),
            false,
            false,
            format!(
                "Nada está no limite, mas o jogo fica em {:.0} fps num monitor de {:.0} Hz: o que segura é o próprio jogo                  (limite dele, do servidor ou do motor). A geração de quadros enche o monitor: é FPS na tela, não do jogo.",
                fps.unwrap_or_default(),
                hz
            ),
        ),
        Gargalo::Nenhum => (
            None,
            None,
            false,
            false,
            "Nada está no limite: o jogo já roda o que esta máquina consegue. O Turbo não liga nada à toa.".to_string(),
        ),
        Gargalo::SemCarga => (
            None,
            None,
            false,
            false,
            "O jogo estava parado durante a medida (menu ou pausa). Jogue e toque em Turbo de novo.".to_string(),
        ),
    };
    // O que faltou na medida vai junto, em vez de a receita parecer feita com tudo medido.
    let mut explicacao = explicacao;
    if fps.is_none() && escala.is_some() {
        explicacao.push_str(" Sem o FPS do jogo (o PresentMon não mediu): o nível da escala é o padrão, não o calculado.");
    }
    if hz_lido.filter(|h| *h > 0).is_none() && multiplicador.is_some() {
        explicacao.push_str(" Não consegui ler o monitor: contei 60 Hz.");
    }
    Receita {
        gargalo,
        escala,
        multiplicador,
        energia,
        memoria,
        explicacao,
    }
}

/// Pura: o aviso quando a receita põe o jogo em janela (a escala) e ele chega à tela POR CÓPIA, o caminho com mais
/// atraso (medido pelo PresentMon nos 10 s da medida). A otimização de janela do Windows (`SwapEffectUpgradeEnable`) não
/// tira a cópia, medido na 3.3.1: o que tira é o jogo apresentar pelo modelo novo, o do DirectX 12 e do Vulkan.
pub fn aviso_de_copia(
    apresentacao: crate::modules::windows::presentmon::Apresentacao,
    api: crate::modules::windows::presentmon::ApiGrafica,
    com_escala: bool,
) -> Option<&'static str> {
    use crate::modules::windows::presentmon::{Apresentacao, ApiGrafica};
    if !com_escala || apresentacao != Apresentacao::Copiada {
        return None;
    }
    Some(match api {
        ApiGrafica::DirectX | ApiGrafica::DirectX9 => {
            "O jogo chega à tela por cópia (DirectX 11 ou anterior em janela), o caminho com mais atraso. Se ele tiver \
             a opção, troque para DirectX 12 ou Vulkan no menu de vídeo: os dois apresentam pelo caminho novo, sem a \
             cópia."
        }
        _ => "O jogo chega à tela por cópia, o caminho com mais atraso. A escala continua dando FPS, mas o clique \
              demora um pouco mais para aparecer.",
    })
}

/// A receita que deu certo num jogo, nesta máquina: na próxima vez, um clique e sem medir de novo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Perfil {
    pub receita: Receita,
    /// O FPS do jogo na medida (o "antes").
    pub fps_antes: Option<f64>,
    pub medido_em: u64,
    /// O desfecho da última prova A-B desta receita. `Piorou`: o Turbo não a liga de novo neste jogo.
    #[serde(default)]
    pub prova: Option<Desfecho>,
}

// ------------------------------------------------------------------------------------------------------------- a prova

/// A prova do Turbo: 8 rodadas alternadas (a ordem da `provaalternada`), com e sem o Turbo, pelo FPS do jogo. Mais
/// curta que a do plano de energia porque o jogo já está aberto e aquecido quando o Turbo foi ligado; a espera é maior
/// porque a escala redimensiona a janela a cada troca.
pub const AQUECIMENTO_DA_PROVA_S: u64 = 30;
pub const ESPERA_DA_PROVA_S: u64 = 6;
pub const RODADA_DA_PROVA_S: u64 = 30;

pub fn duracao_da_prova_s() -> u64 {
    AQUECIMENTO_DA_PROVA_S + ORDEM.len() as u64 * (ESPERA_DA_PROVA_S + RODADA_DA_PROVA_S)
}

/// Pura: o que entra na prova. Só o que deve subir o FPS DO JOGO: a escala (sem gerar quadros) e a energia de jogo. A
/// geração aumenta o que chega à tela e custa um pouco do jogo; ela é julgada pela guarda do gerador, não aqui (pelo
/// FPS do jogo, uma geração boa sairia como "piorou"). `None`: a receita não tem o que provar no FPS do jogo.
/// Pura: a receita lembrada liga pela primeira vez sem prova e muda o FPS do jogo. Então o Turbo se prova sozinho e
/// desliga se piorar: "nunca menos FPS" sem depender de a pessoa lembrar de tocar em Provar.
pub fn prova_sozinha(p: &Perfil) -> bool {
    p.prova.is_none() && receita_da_prova(&p.receita).is_some()
}

pub fn receita_da_prova(r: &Receita) -> Option<Receita> {
    if r.escala.is_none() && !r.energia {
        return None;
    }
    Some(Receita {
        multiplicador: r.escala.map(|_| 1),
        memoria: false,
        ..r.clone()
    })
}

/// Pura: em palavras, o que a prova ligou ("Turbo: escala Qualidade + energia de jogo").
pub fn descrever(r: &Receita) -> String {
    let mut partes = Vec::new();
    if let Some(n) = r.escala {
        partes.push(format!("escala {}", n.nome()));
    }
    if r.energia {
        partes.push("energia de jogo".to_string());
    }
    format!("Turbo: {}", partes.join(" + "))
}

/// Pura: a frase da tela para o desfecho, nas palavras do Turbo. Sem comparação justa, a explicação da
/// `provaalternada` (o que mudou no meio) já serve.
pub fn leitura_da_prova(d: Desfecho, sem_comparacao: &str) -> String {
    match d {
        Desfecho::Ganhou => "Com o Turbo o jogo rodou melhor neste PC, e a diferença passou do ruído nas rodadas \
                             alternadas. O Turbo fica ligado."
            .to_string(),
        Desfecho::Piorou => "Com o Turbo o jogo rodou pior neste PC, além do ruído. O Turbo foi desligado e não liga \
                             de novo esta receita neste jogo."
            .to_string(),
        Desfecho::Indistinguivel => "As rodadas não separaram com e sem o Turbo: a diferença ficou dentro do ruído do \
                                     jogo. Não piorou, então o Turbo fica; o ganho dele aqui não foi comprovado."
            .to_string(),
        Desfecho::SemComparacaoJusta => sem_comparacao.to_string(),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Perfis {
    /// Pelo id do jogo (`historico::id_do_jogo`).
    jogos: BTreeMap<String, Perfil>,
    /// "Menor atraso" (sem geração) ou "Mais FPS", escolhido no cartão. Sem escolha, vale a sugestão.
    menor_atraso: BTreeMap<String, bool>,
}

fn caminho() -> PathBuf {
    std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("turbo.json")
}

fn ler_em(c: &Path) -> Perfis {
    fs::read_to_string(c)
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .unwrap_or_default()
}

fn gravar_em(c: &Path, p: &Perfis) -> Result<(), String> {
    if let Some(d) = c.parent() {
        fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let texto = serde_json::to_string_pretty(p).map_err(|e| e.to_string())?;
    fs::write(c, texto).map_err(|e| e.to_string())
}

pub fn perfil(id_do_jogo: &str) -> Option<Perfil> {
    ler_em(&caminho()).jogos.get(id_do_jogo).cloned()
}

pub fn lembrar(id_do_jogo: &str, perfil: Perfil) -> Result<(), String> {
    let c = caminho();
    let mut p = ler_em(&c);
    p.jogos.insert(id_do_jogo.to_string(), perfil);
    gravar_em(&c, &p)
}

pub fn menor_atraso(id_do_jogo: &str) -> bool {
    menor_atraso_em(&ler_em(&caminho()), id_do_jogo)
}

/// Guarda a escolha. A receita lembrada foi montada com a outra e é esquecida (o próximo Turbo mede de novo); a
/// reprovada fica, para o "Medir de novo" continuar sem religá-la.
pub fn escolher_atraso(id_do_jogo: &str, menor: bool) -> Result<(), String> {
    let c = caminho();
    let mut p = ler_em(&c);
    escolher_atraso_em(&mut p, id_do_jogo, menor);
    gravar_em(&c, &p)
}

fn escolher_atraso_em(p: &mut Perfis, id_do_jogo: &str, menor: bool) {
    if menor_atraso_em(p, id_do_jogo) != menor
        && p.jogos.get(id_do_jogo).is_some_and(|x| x.prova != Some(Desfecho::Piorou))
    {
        p.jogos.remove(id_do_jogo);
    }
    p.menor_atraso.insert(id_do_jogo.to_string(), menor);
}

pub fn esquecer(id_do_jogo: &str) -> Result<(), String> {
    let c = caminho();
    let mut p = ler_em(&c);
    if p.jogos.remove(id_do_jogo).is_some() {
        gravar_em(&c, &p)?;
    }
    Ok(())
}

/// O antes × depois da última vez que o Turbo ficou ligado com o Frame Boost. NÃO é prova A-B: o "antes" são os 10 s
/// da medida e o "depois" a média da sessão (cenas diferentes). A tela diz isso; o compartilhar fica para a prova.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Resultado {
    pub fps_antes: f64,
    /// FPS do jogo com o Turbo, média da sessão.
    pub fps_do_jogo: f64,
    pub ganho_pct: f64,
    /// Só com geração: o que chegou à tela.
    pub fps_na_tela: Option<f64>,
    pub na_tela_medido: bool,
    pub minutos: f64,
    pub escala: Option<Nivel>,
}

/// Pura. Sem o antes, sem sessão (menos de 30 s gerando) ou sem Frame Boost: nada a mostrar.
pub fn resultado_de(
    fps_antes: Option<f64>,
    sessao: Option<&crate::modules::windows::geracao::resumo::Resumo>,
    escala: Option<Nivel>,
    gerou: bool,
) -> Option<Resultado> {
    let antes = fps_antes.filter(|f| *f > 0.0)?;
    let s = sessao.filter(|s| s.fps_do_jogo > 0.0)?;
    Some(Resultado {
        fps_antes: antes.round(),
        fps_do_jogo: s.fps_do_jogo.round(),
        ganho_pct: ((s.fps_do_jogo / antes - 1.0) * 1000.0).round() / 10.0,
        fps_na_tela: gerou.then_some(s.fps_na_tela.round()),
        na_tela_medido: gerou && s.na_tela_medido,
        minutos: (s.segundos / 6.0).round() / 10.0,
        escala,
    })
}

/// Para a tela.
#[derive(Debug, Clone, Serialize)]
pub struct Painel {
    pub jogo: Option<String>,
    /// A escolha do cartão para o jogo aberto (ou a sugestão, sem escolha).
    pub menor_atraso: bool,
    /// O que ficou lembrado para este jogo.
    pub perfil: Option<Perfil>,
    /// O que o Turbo fez da última vez que ligou (nesta abertura do Otimiza).
    pub passos: Vec<String>,
    pub ligado: bool,
    /// Desligado: o antes × depois da última vez.
    pub resultado: Option<Resultado>,
    /// A última prova A-B do Turbo NESTE jogo.
    pub prova: Option<crate::modules::provaalternada::Resultado>,
    /// A prova está rodando agora.
    pub provando: bool,
    /// A que está rodando é a automática, da primeira vez da receita neste jogo.
    pub prova_sozinha: bool,
    /// Quanto a prova leva, para a tela dizer antes de começar.
    pub duracao_da_prova_s: u64,
    /// O jogo aberto trava muitas vezes por minuto (última medida do Turbo): a oferta do Jogo liso.
    pub ritmo: Option<Ritmo>,
}

#[cfg(target_os = "windows")]
static PASSOS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Uma linha a mais no que o Turbo fez (os passos que moram fora daqui: governador, núcleos rápidos).
#[cfg(target_os = "windows")]
pub fn anotar_passo(texto: String) {
    if let Ok(mut p) = PASSOS.lock() {
        p.push(texto);
    }
}

/// O perfil gráfico do GTA (configjogo) pedido no cartão do Turbo: o arquivo só pode ser escrito com o jogo fechado
/// (ele reescreve a configuração ao sair), então o pedido fica em disco e o laço da sessão aplica quando ele fechar.
pub const PERFIS_DO_JOGO: [&str; 2] = ["competitivo", "equilibrado"];

/// Pura: o jogo tem o perfil gráfico do GTA (FiveM e GTA V usam o mesmo settings.xml).
pub fn oferece_perfil_do_jogo(id_do_jogo: &str) -> bool {
    matches!(id_do_jogo, "fivem" | "gta5")
}

fn marca_do_perfil_do_jogo() -> PathBuf {
    caminho().with_file_name("turbo_perfil_do_jogo")
}

/// `None` cancela o pedido.
pub fn pedir_perfil_do_jogo(perfil: Option<&str>) -> Result<(), String> {
    let m = marca_do_perfil_do_jogo();
    match perfil {
        Some(p) if PERFIS_DO_JOGO.contains(&p) => {
            if let Some(d) = m.parent() {
                fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            fs::write(&m, p).map_err(|e| e.to_string())
        }
        Some(outro) => Err(format!("perfil desconhecido: {}", outro)),
        None => match fs::remove_file(&m) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        },
    }
}

pub fn perfil_do_jogo_pendente() -> Option<String> {
    fs::read_to_string(marca_do_perfil_do_jogo())
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|p| PERFIS_DO_JOGO.contains(&p.as_str()))
}

/// A oferta do Jogo liso no cartão do Turbo: o jogo trava muitas vezes por minuto e um teto estável tira o tranco.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ritmo {
    pub fps: f64,
    pub trancos_por_minuto: f64,
    pub teto: u32,
}

/// Pura: o Turbo OFERECE o teto do Jogo liso (não aplica: o teto troca um pouco da média por ritmo, e essa troca é da
/// pessoa) quando a medida mostra trancos e a receita não mexe no FPS depois da medida. Com escala, o FPS sobe depois
/// e o teto da medida cortaria o ganho; com geração, a base já é travada pelo Frame Boost (`travar_base`).
pub fn ritmo_a_oferecer(r: &Receita, mediana: f64, trancos_por_minuto: f64, hz: u32) -> Option<Ritmo> {
    if r.escala.is_some() || r.multiplicador.is_some() || trancos_por_minuto < super::jogo_liso::JA_LISO_TRANCOS {
        return None;
    }
    super::jogo_liso::planejar(mediana, hz, trancos_por_minuto)
        .teto
        .map(|teto| Ritmo { fps: mediana, trancos_por_minuto, teto })
}

/// Pura: com o gargalo no processador ou na memória, o que disputa com o jogo é o segundo plano: o governador do modo
/// jogo entra (ele tem o próprio portão "nunca menos FPS" e para sozinho se a partida piorar).
pub fn governador_na_receita(r: &Receita) -> bool {
    matches!(r.gargalo, Gargalo::Processador | Gargalo::Memoria)
}

/// Pura: depois de uma prova que não piorou, com o processador no limite e sem regra de núcleos para o jogo, o Turbo
/// mede os núcleos rápidos (o Auto CPU Set, que só fica se render).
pub fn testar_nucleos(r: &Receita, desfecho: Desfecho, ja_tem_regra: bool) -> bool {
    r.gargalo == Gargalo::Processador && desfecho != Desfecho::Piorou && !ja_tem_regra
}

/// O Turbo ligou o governador (e não a pessoa, com o modo jogo automático): o vigia do modo jogo roda enquanto isto
/// vale e o devolve quando o jogo fecha; desligar o Turbo também. Em memória: se o Otimiza cair, a abertura seguinte
/// devolve pelo arquivo do governador (`gamemode::recuperar_na_abertura`).
#[cfg(target_os = "windows")]
static GOVERNA: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "windows")]
pub fn governa() -> bool {
    GOVERNA.load(std::sync::atomic::Ordering::SeqCst)
}

#[cfg(target_os = "windows")]
pub fn marcar_governo(ligado: bool) {
    GOVERNA.store(ligado, std::sync::atomic::Ordering::SeqCst);
}
/// O Turbo ligou a energia de jogo (e não a pessoa): desligar o Turbo devolve só o que ele ligou. Em disco, ao lado
/// do turbo.json: depois de fechar e abrir o Otimiza, o "Desligar Turbo" ainda sabe que a energia foi dele.
#[cfg(target_os = "windows")]
fn marca_da_energia() -> PathBuf {
    caminho().with_file_name("turbo_energia")
}

#[cfg(target_os = "windows")]
fn ligou_a_energia() -> bool {
    marca_da_energia().exists()
}

#[cfg(target_os = "windows")]
fn marcar_energia(ligou: bool) {
    let m = marca_da_energia();
    let _ = if ligou {
        m.parent().map(std::fs::create_dir_all);
        std::fs::write(&m, b"")
    } else {
        std::fs::remove_file(&m)
    };
}
#[cfg(target_os = "windows")]
static LIGADO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// O que o resultado precisa: o FPS de antes e a receita aplicada (ao ligar), e o antes × depois (ao desligar).
#[cfg(target_os = "windows")]
static APLICADA: std::sync::Mutex<Option<(Option<f64>, Receita)>> = std::sync::Mutex::new(None);
#[cfg(target_os = "windows")]
static RESULTADO: std::sync::Mutex<Option<Resultado>> = std::sync::Mutex::new(None);

#[cfg(target_os = "windows")]
pub fn painel() -> Painel {
    use std::sync::atomic::Ordering;
    let jogo = super::sessao::atual();
    let id = jogo
        .as_ref()
        .map(|j| super::historico::id_do_jogo(&j.executavel).0);
    Painel {
        jogo: jogo.as_ref().map(|j| j.nome.clone()),
        menor_atraso: id.as_deref().is_some_and(menor_atraso),
        perfil: id.as_deref().and_then(perfil),
        passos: PASSOS.lock().map(|p| p.clone()).unwrap_or_default(),
        ligado: LIGADO.load(Ordering::SeqCst),
        resultado: (!LIGADO.load(Ordering::SeqCst))
            .then(|| RESULTADO.lock().ok().and_then(|r| r.clone()))
            .flatten(),
        prova: jogo.as_ref().and_then(|j| {
            prova_guardada().filter(|p| {
                super::historico::id_do_jogo(&p.jogo).0 == super::historico::id_do_jogo(&j.executavel).0
            })
        }),
        provando: em_prova() || PROVA_SOZINHA.load(Ordering::SeqCst),
        prova_sozinha: PROVA_SOZINHA.load(Ordering::SeqCst),
        duracao_da_prova_s: duracao_da_prova_s(),
        ritmo: RITMO
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .filter(|(jogo, _)| id.as_deref() == Some(jogo.as_str()))
            .map(|(_, r)| r),
    }
}

/// Mede 10 s (gargalo pelos contadores do Windows e FPS pelo PresentMon, ao mesmo tempo) e monta a receita. Bloqueia.
#[cfg(target_os = "windows")]
pub fn medir() -> Result<(Receita, Option<f64>), String> {
    use crate::modules::windows::{bottleneck, presentmon};
    let jogo = super::sessao::atual().ok_or("Abra o jogo primeiro: o Turbo mede o jogo aberto.")?;
    let pid = jogo.pid;
    let medida = std::thread::spawn(move || {
        presentmon::capturar(pid, 10)
            .and_then(|q| presentmon::resumir(&q))
            .map_err(|e| crate::utils::Logger::warn(&format!("turbo: o PresentMon não mediu o jogo: {}", e)))
            .ok()
            .filter(|r| r.fps_do_jogo_medio > 0.0)
    });
    let relatorio = bottleneck::analisar(10);
    let medida = medida.join().ok().flatten();
    let fps = medida.as_ref().map(|r| r.fps_do_jogo_medio);
    let (id, _) = super::historico::id_do_jogo(&jogo.executavel);
    let hz = super::frameboost::hz_do_jogo(pid);
    crate::utils::Logger::info(&format!(
        "turbo: {} · {:?} (CPU {:.0}%, núcleo {:.0}%, placa {:.0}%) · {:?} fps · {:?} Hz",
        id, relatorio.limite, relatorio.cpu_total, relatorio.cpu_max_core, relatorio.gpu_percent, fps, hz
    ));
    let receita = montar(gargalo_de(relatorio.limite), fps, hz, menor_atraso(&id));
    let ritmo = medida.as_ref().and_then(|r| {
        ritmo_a_oferecer(&receita, r.fps_do_jogo_mediana, r.engasgos_por_minuto, hz.unwrap_or(0))
    });
    if let Ok(mut g) = RITMO.lock() {
        *g = ritmo.map(|r| (id.clone(), r));
    }
    Ok((receita, fps))
}

/// O ritmo irregular da última medida do Turbo, por jogo, para o cartão oferecer o Jogo liso.
#[cfg(target_os = "windows")]
static RITMO: std::sync::Mutex<Option<(String, Ritmo)>> = std::sync::Mutex::new(None);

/// Aplica a receita e devolve o que fez, passo a passo. Um passo que falha (anticheat, sem administrador) vira nota e
/// os outros seguem: o Turbo nunca deixa de fazer o que pode por causa do que não pode.
#[cfg(target_os = "windows")]
pub fn aplicar(app: &tauri::AppHandle, r: &Receita, fps_antes: Option<f64>) -> Vec<String> {
    use std::sync::atomic::Ordering;
    if let Ok(mut a) = APLICADA.lock() {
        *a = Some((fps_antes, r.clone()));
    }
    let mut passos = vec![r.explicacao.clone()];
    if r.energia {
        let ja = super::energia_jogo::ler().ligado;
        if ja {
            passos.push("Energia de jogo: já estava ligada.".to_string());
        } else {
            match super::energia_jogo::definir(true) {
                Ok(_) => {
                    marcar_energia(true);
                    passos.push("Energia de jogo: ligada.".to_string());
                }
                Err(e) => passos.push(format!("Energia de jogo: {}", e)),
            }
        }
    }
    if let Some(m) = r.multiplicador {
        match super::frameboost::ligar(app, m, false, r.escala) {
            Ok(_) => passos.push(match (r.escala, m) {
                (Some(n), 1) => format!("Escala {:?}: ligada, sem geração.", n),
                (Some(n), _) => format!("Escala {:?} e geração de quadros: ligadas.", n),
                (None, _) => "Geração de quadros: ligada.".to_string(),
            }),
            Err(e) => passos.push(format!("Frame Boost: {}", e)),
        }
    }
    if r.memoria {
        passos.push("Memória: veja abaixo o que ocupa e feche o que não precisa.".to_string());
    }
    // Com a escala, o jogo foi para janela: o modo de apresentação de agora é o que vale (o da medida era em tela
    // cheia). Medido em 3 s, depois de a janela se ajustar.
    if r.escala.is_some() && super::frameboost::ativo_de(&crate::modules::windows::geracao::estado()).is_some() {
        if let Some(j) = super::sessao::atual() {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            let medido = crate::modules::windows::presentmon::capturar(j.pid, 3)
                .and_then(|q| crate::modules::windows::presentmon::resumir(&q));
            if let Ok(resumo) = medido {
                if let Some(aviso) = aviso_de_copia(resumo.apresentacao, resumo.api, true) {
                    passos.push(aviso.to_string());
                }
            }
        }
    }
    LIGADO.store(true, Ordering::SeqCst);
    if let Ok(mut p) = PASSOS.lock() {
        *p = passos.clone();
    }
    passos
}

/// Desfaz o que o Turbo ligou: o Frame Boost e, se foi ele que ligou, a energia de jogo.
#[cfg(target_os = "windows")]
pub fn desligar() -> Result<(), String> {
    use std::sync::atomic::Ordering;
    super::frameboost::desligar()?;
    // A sessão do gerador continua no estado depois de desligar (só zera ao ligar de novo).
    let aplicada = APLICADA.lock().ok().and_then(|mut a| a.take());
    if let Some((fps_antes, receita)) = aplicada.filter(|(_, r)| r.multiplicador.is_some()) {
        let sessao = crate::modules::windows::geracao::estado().sessao.resumo();
        let gerou = receita.multiplicador != Some(1);
        if let Ok(mut r) = RESULTADO.lock() {
            *r = resultado_de(fps_antes, sessao.as_ref(), receita.escala, gerou);
        }
    }
    if ligou_a_energia() {
        super::energia_jogo::definir(false)?;
        marcar_energia(false);
    }
    LIGADO.store(false, Ordering::SeqCst);
    if let Ok(mut p) = PASSOS.lock() {
        p.clear();
    }
    Ok(())
}

/// Um passo da prova, para a tela: 0 é o aquecimento; 1 a 8, as rodadas.
#[derive(Debug, Clone, Serialize)]
pub struct PassoDaProva {
    pub indice: usize,
    pub total: usize,
    pub com_turbo: Option<bool>,
    pub segundos: u64,
}

/// Em disco durante a prova: se o Otimiza cair no meio, a abertura devolve a energia de jogo como estava.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ProvaEmAndamento {
    energia_antes: bool,
}

fn pasta() -> PathBuf {
    caminho().parent().map(Path::to_path_buf).unwrap_or_default()
}

fn marcador_da_prova() -> PathBuf {
    pasta().join("turbo_prova_em_andamento.json")
}

/// A última prova do Turbo (para a tela e o compartilhar).
pub fn prova_guardada() -> Option<crate::modules::provaalternada::Resultado> {
    fs::read_to_string(pasta().join("turbo_prova.json"))
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
}

fn guardar_prova(r: &crate::modules::provaalternada::Resultado) -> Result<(), String> {
    let texto = serde_json::to_string_pretty(r).map_err(|e| e.to_string())?;
    fs::create_dir_all(pasta()).map_err(|e| e.to_string())?;
    fs::write(pasta().join("turbo_prova.json"), texto).map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
static EM_PROVA: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// A prova automática foi pedida e ainda não acabou (cobre também o instante antes de a `provar` marcar a `EM_PROVA`).
#[cfg(target_os = "windows")]
static PROVA_SOZINHA: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A vez da prova automática, de uma vez só (dois "ligar" seguidos não abrem duas). `false`: já há prova.
#[cfg(target_os = "windows")]
pub fn reservar_prova_sozinha() -> bool {
    use std::sync::atomic::Ordering;
    !em_prova()
        && PROVA_SOZINHA
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
}

/// Só quem reservou libera.
#[cfg(target_os = "windows")]
pub fn liberar_prova_sozinha() {
    PROVA_SOZINHA.store(false, std::sync::atomic::Ordering::SeqCst);
}

/// A prova automática ainda vai começar ou está rodando: o Turbo não se liga nem se desliga por cima dela.
#[cfg(target_os = "windows")]
pub fn prova_sozinha_pendente() -> bool {
    PROVA_SOZINHA.load(std::sync::atomic::Ordering::SeqCst)
}

#[cfg(target_os = "windows")]
pub fn em_prova() -> bool {
    EM_PROVA.load(std::sync::atomic::Ordering::SeqCst)
}

/// O Otimiza caiu no meio da prova: a energia de jogo volta a como estava antes dela.
#[cfg(target_os = "windows")]
pub fn recuperar_na_abertura() -> Option<String> {
    let texto = fs::read_to_string(marcador_da_prova()).ok()?;
    let _ = fs::remove_file(marcador_da_prova());
    let m: ProvaEmAndamento = serde_json::from_str(&texto).ok()?;
    if !m.energia_antes && super::energia_jogo::ler().ligado {
        let _ = super::energia_jogo::definir(false);
        return Some("energia de jogo desligada, como estava antes da prova".to_string());
    }
    Some("nada a devolver".to_string())
}

/// Liga o lado "com o Turbo" da prova (sem geração: `receita_da_prova`).
#[cfg(target_os = "windows")]
fn ligar_o_lado(app: &tauri::AppHandle, r: &Receita) -> Result<(), String> {
    if r.energia && !super::energia_jogo::ler().ligado {
        super::energia_jogo::definir(true).map(|_| ())?;
    }
    if let Some(m) = r.multiplicador {
        super::frameboost::ligar(app, m, false, r.escala).map(|_| ())?;
    }
    Ok(())
}

/// Volta ao lado "sem o Turbo": o Frame Boost desligado (a janela do jogo volta ao tamanho) e a energia como estava.
#[cfg(target_os = "windows")]
fn desligar_o_lado(energia_antes: bool) -> Result<(), String> {
    super::frameboost::desligar()?;
    if !energia_antes && super::energia_jogo::ler().ligado {
        super::energia_jogo::definir(false).map(|_| ())?;
    }
    Ok(())
}

/// Saindo da prova por qualquer caminho: o marcador fora e a vez liberada. Armada (erro, jogo fechou no meio), também
/// o lado "sem"; desarmada no fim, que já deixou o Turbo como o desfecho manda.
#[cfg(target_os = "windows")]
struct FimDaProva {
    energia_antes: bool,
    armado: bool,
}

#[cfg(target_os = "windows")]
impl Drop for FimDaProva {
    fn drop(&mut self) {
        if self.armado {
            let _ = desligar_o_lado(self.energia_antes);
        }
        let _ = fs::remove_file(marcador_da_prova());
        EM_PROVA.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// A PROVA DO TURBO. Com o jogo aberto, alterna o Turbo (escala e energia, sem geração) em 8 rodadas, mede o FPS do jogo
/// em cada uma e decide pelo critério único da `provaalternada`. Piorou: o Turbo fica desligado e a receita não volta
/// neste jogo. Senão: a receita inteira (com a geração, se tiver) é ligada de novo. Bloqueia por uns 5 minutos.
#[cfg(target_os = "windows")]
pub fn provar(
    app: &tauri::AppHandle,
    emitir: &dyn Fn(PassoDaProva),
) -> Result<crate::modules::provaalternada::Resultado, String> {
    use crate::modules::provaalternada as alternada;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let jogo = super::sessao::atual().ok_or("Abra o jogo primeiro: a prova mede o jogo aberto.")?;
    let (id, _) = super::historico::id_do_jogo(&jogo.executavel);
    let mut perfil = perfil(&id)
        .ok_or("Ligue o Turbo uma vez neste jogo: a prova usa a receita que ele escolheu para ele.")?;
    let lado_com = receita_da_prova(&perfil.receita).ok_or(
        "Esta receita não muda o FPS do jogo (só a geração de quadros ou a memória): não há o que provar com o FPS \
         do jogo. A guarda do Frame Boost já confere a geração.",
    )?;
    // A vez da prova alternada: a medição automática e o resto que respeita a prova esperam esta também.
    let _vez = alternada::reservar().ok_or("Já há uma prova rodando.")?;
    if EM_PROVA.swap(true, Ordering::SeqCst) {
        return Err("Já há uma prova rodando.".to_string());
    }
    let energia_antes = super::energia_jogo::ler().ligado
        && !ligou_a_energia();
    // A partir daqui, qualquer saída devolve o lado "sem" e libera a vez.
    let mut fim = FimDaProva {
        energia_antes,
        armado: true,
    };
    let _ = desligar();
    if let Ok(t) = serde_json::to_string(&ProvaEmAndamento { energia_antes }) {
        let _ = fs::create_dir_all(pasta());
        let _ = fs::write(marcador_da_prova(), t);
    }

    let pid = jogo.pid;
    let nome = jogo.executavel.clone();
    let aberto = || super::sessao::atual().is_some_and(|j| j.pid == pid);
    let total = ORDEM.len();
    emitir(PassoDaProva {
        indice: 0,
        total,
        com_turbo: None,
        segundos: AQUECIMENTO_DA_PROVA_S,
    });
    for _ in 0..AQUECIMENTO_DA_PROVA_S {
        std::thread::sleep(Duration::from_secs(1));
        if !aberto() {
            return Err("O jogo fechou durante o aquecimento. Nada foi medido.".to_string());
        }
    }

    let mut rodadas = Vec::with_capacity(total);
    for (i, lado) in ORDEM.iter().enumerate() {
        let com = *lado == alternada::Lado::ComOtimiza;
        if com {
            ligar_o_lado(app, &lado_com)?;
        } else {
            desligar_o_lado(energia_antes)?;
        }
        emitir(PassoDaProva {
            indice: i + 1,
            total,
            com_turbo: Some(com),
            segundos: RODADA_DA_PROVA_S,
        });
        std::thread::sleep(Duration::from_secs(ESPERA_DA_PROVA_S));
        if !aberto() {
            return Err("O jogo fechou no meio da prova. O Turbo ficou desligado.".to_string());
        }
        let rodada = alternada::medir(pid, &nome, *lado, RODADA_DA_PROVA_S)?;
        if !aberto() {
            return Err("O jogo fechou no meio da prova. O Turbo ficou desligado.".to_string());
        }
        crate::utils::Logger::info(&format!(
            "prova do Turbo: rodada {}/{} {}: {:.1} FPS do jogo, 1% {:?}",
            i + 1,
            total,
            if com { "com" } else { "sem" },
            rodada.fps_do_jogo,
            rodada.low_1pct
        ));
        rodadas.push(rodada);
    }
    desligar_o_lado(energia_antes)?;
    fim.armado = false;

    let mut r = alternada::decidir(
        &nome,
        crate::modules::changelog::now_timestamp(),
        RODADA_DA_PROVA_S,
        rodadas,
        false,
    );
    r.leitura = leitura_da_prova(r.desfecho, &r.leitura);
    if r.desfecho != Desfecho::Piorou {
        // A receita inteira volta, com a geração se tiver.
        aplicar(app, &perfil.receita, perfil.fps_antes);
    }
    perfil.prova = Some(r.desfecho);
    if let Err(e) = lembrar(&id, perfil) {
        crate::utils::Logger::warn(&format!("prova do Turbo: não guardei o desfecho: {}", e));
    }
    if let Err(e) = guardar_prova(&r) {
        crate::utils::Logger::warn(&format!("prova do Turbo: não guardei o resultado: {}", e));
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_perfil_do_gta_so_e_oferecido_no_fivem_e_no_gta() {
        assert!(oferece_perfil_do_jogo("fivem") && oferece_perfil_do_jogo("gta5"));
        assert!(!oferece_perfil_do_jogo("cs2"));
        assert!(PERFIS_DO_JOGO.iter().all(|p| ["competitivo", "equilibrado"].contains(p)));
    }

    #[test]
    fn o_jogo_liso_so_e_oferecido_com_tranco_e_sem_escala_nem_geracao() {
        let cpu_menor_atraso = montar(Gargalo::Processador, Some(70.0), Some(144), true);
        let r = ritmo_a_oferecer(&cpu_menor_atraso, 70.0, 30.0, 144).expect("30 trancos/min sem escala nem geração");
        assert!(r.teto < 70, "o teto fica abaixo da mediana, com folga para o processador: {}", r.teto);
        assert!(ritmo_a_oferecer(&cpu_menor_atraso, 70.0, 1.0, 144).is_none(), "já liso");
        let com_geracao = montar(Gargalo::Processador, Some(70.0), Some(144), false);
        assert!(ritmo_a_oferecer(&com_geracao, 70.0, 30.0, 144).is_none(), "a geração já trava a base");
        let com_escala = montar(Gargalo::Placa, Some(70.0), Some(144), true);
        assert!(ritmo_a_oferecer(&com_escala, 70.0, 30.0, 144).is_none(), "a escala sobe o FPS depois da medida");
    }

    #[test]
    fn o_governador_e_os_nucleos_so_entram_onde_o_processador_disputa() {
        let placa = montar(Gargalo::Placa, Some(40.0), Some(144), false);
        let cpu = montar(Gargalo::Processador, Some(40.0), Some(144), false);
        let memoria = montar(Gargalo::Memoria, Some(40.0), Some(144), false);
        assert!(!governador_na_receita(&placa), "a placa no limite: o segundo plano não é o problema");
        assert!(governador_na_receita(&cpu) && governador_na_receita(&memoria));

        assert!(testar_nucleos(&cpu, Desfecho::Ganhou, false));
        assert!(testar_nucleos(&cpu, Desfecho::Indistinguivel, false));
        assert!(!testar_nucleos(&cpu, Desfecho::Piorou, false), "o Turbo saiu: nada mais entra");
        assert!(!testar_nucleos(&cpu, Desfecho::Ganhou, true), "já medido neste jogo: o vigia reaplica");
        assert!(!testar_nucleos(&placa, Desfecho::Ganhou, false));
    }

    #[test]
    fn so_se_prova_sozinha_a_receita_nunca_provada_que_muda_o_fps() {
        let com_escala = montar(Gargalo::Placa, Some(40.0), Some(144), true);
        let perfil = |receita: Receita, prova| Perfil { receita, fps_antes: Some(40.0), medido_em: 1, prova };
        assert!(prova_sozinha(&perfil(com_escala.clone(), None)));
        for ja in [Desfecho::Ganhou, Desfecho::Piorou, Desfecho::Indistinguivel, Desfecho::SemComparacaoJusta] {
            assert!(!prova_sozinha(&perfil(com_escala.clone(), Some(ja))), "{ja:?}: já provada");
        }
        let nada = montar(Gargalo::Nenhum, Some(200.0), Some(144), false);
        assert!(!prova_sozinha(&perfil(nada, None)), "não muda o FPS do jogo: não há o que provar");
    }

    #[test]
    fn a_mesma_acao_ignora_o_texto_e_o_gargalo() {
        let base = Receita {
            gargalo: Gargalo::Placa,
            escala: Some(Nivel::Qualidade),
            multiplicador: Some(1),
            energia: true,
            memoria: false,
            explicacao: "a 60 fps".into(),
        };
        let outra_medida = Receita { explicacao: "a 58 fps".into(), memoria: true, ..base.clone() };
        assert!(mesma_acao(&base, &outra_medida));
        assert!(!mesma_acao(&base, &Receita { escala: Some(Nivel::Desempenho), ..base.clone() }));
        assert!(!mesma_acao(&base, &Receita { energia: false, ..base.clone() }));
        assert!(!mesma_acao(&base, &Receita { multiplicador: Some(2), ..base.clone() }));
    }

    #[test]
    fn cada_limite_tem_o_seu_gargalo() {
        assert_eq!(gargalo_de(Limite::Gpu), Gargalo::Placa);
        assert_eq!(gargalo_de(Limite::MemoriaVideo), Gargalo::Placa);
        assert_eq!(gargalo_de(Limite::CpuUmNucleo), Gargalo::Processador);
        assert_eq!(gargalo_de(Limite::CpuTodos), Gargalo::Processador);
        assert_eq!(gargalo_de(Limite::MemoriaRam), Gargalo::Memoria);
        assert_eq!(gargalo_de(Limite::SemCarga), Gargalo::SemCarga);
        assert_eq!(gargalo_de(Limite::NaoIdentificado), Gargalo::Nenhum);
    }

    #[test]
    fn o_nivel_e_o_mais_bonito_que_alcanca() {
        // 60 fps num monitor de 144: a Ultra (1,3×) dá ~87, a Qualidade (1,5×) ~106, o Equilibrado ~126 e o
        // Desempenho ~158. Só o Desempenho chega a 144.
        assert_eq!(nivel_automatico(Some(60.0), 144.0), Nivel::Desempenho);
        assert_eq!(nivel_automatico(Some(60.0), 72.0), Nivel::UltraQualidade);
        assert_eq!(nivel_automatico(Some(60.0), 100.0), Nivel::Qualidade);
        assert_eq!(nivel_automatico(None, 144.0), Nivel::Qualidade, "sem medida, o padrão");
    }

    #[test]
    fn o_ganho_estimado_e_menor_que_o_dos_pixels() {
        let f = fps_no_nivel(100.0, Nivel::Qualidade);
        assert!(f > 150.0 && f < 225.0, "{}", f);
    }

    #[test]
    fn placa_no_limite_ganha_escala_e_geracao_se_faltar() {
        let r = montar(Gargalo::Placa, Some(40.0), Some(144), false);
        assert!(r.escala.is_some());
        assert_eq!(r.multiplicador, Some(0), "40 fps não enche 144 Hz só com escala: Auto");
        assert!(r.energia);
    }

    #[test]
    fn placa_no_limite_perto_do_monitor_fica_so_com_escala() {
        let r = montar(Gargalo::Placa, Some(110.0), Some(144), false);
        assert_eq!(r.multiplicador, Some(1), "a escala já leva 110 a 144: sem geração");
        assert!(r.escala.is_some());
    }

    #[test]
    fn competitivo_nunca_ganha_geracao() {
        let r = montar(Gargalo::Placa, Some(40.0), Some(240), true);
        assert_eq!(r.multiplicador, Some(1));
        let r = montar(Gargalo::Processador, Some(40.0), Some(240), true);
        assert_eq!(r.multiplicador, None);
        assert!(r.energia);
    }

    #[test]
    fn processador_no_limite_nao_ganha_escala() {
        let r = montar(Gargalo::Processador, Some(45.0), Some(180), false);
        assert_eq!(r.escala, None, "a placa já sobra: menos pixels não dá FPS");
        assert_eq!(r.multiplicador, Some(0));
        assert!(r.energia);
    }

    #[test]
    fn memoria_acabando_oferece_a_memoria_e_nao_liga_o_frame_boost() {
        let r = montar(Gargalo::Memoria, Some(45.0), Some(60), false);
        assert!(r.memoria);
        assert_eq!(r.multiplicador, None);
    }

    #[test]
    fn nada_no_limite_com_o_monitor_sobrando_enche_o_monitor_com_geracao() {
        let r = montar(Gargalo::Nenhum, Some(70.0), Some(144), false);
        assert_eq!((r.escala, r.multiplicador, r.energia), (None, Some(0), false));
        assert!(r.explicacao.contains("FPS na tela"), "a geração nunca é vendida como FPS do jogo");
        let menor_atraso = montar(Gargalo::Nenhum, Some(70.0), Some(144), true);
        assert_eq!(menor_atraso.multiplicador, None, "Menor atraso: sem geração");
        let sem_medida = montar(Gargalo::Nenhum, None, Some(144), false);
        assert_eq!(sem_medida.multiplicador, None, "sem o FPS medido, não há base para dizer que sobra monitor");
        let perto = montar(Gargalo::Nenhum, Some(135.0), Some(144), false);
        assert_eq!(perto.multiplicador, None, "a 94% dos Hz o monitor já está quase cheio");
        assert!(receita_da_prova(&r).is_none(), "só geração: a guarda do Frame Boost confere, não a prova");
    }

    #[test]
    fn nada_no_limite_nao_liga_nada() {
        for g in [Gargalo::Nenhum, Gargalo::SemCarga] {
            let r = montar(g, Some(200.0), Some(144), false);
            assert_eq!((r.escala, r.multiplicador, r.energia, r.memoria), (None, None, false, false));
            assert!(!r.explicacao.is_empty());
        }
    }

    #[test]
    fn a_escolha_do_atraso_vence_a_sugestao_e_esquece_a_receita_da_outra() {
        let receita = montar(Gargalo::Placa, Some(40.0), Some(144), false);
        let perfil = |prova| Perfil { receita: receita.clone(), fps_antes: Some(40.0), medido_em: 1, prova };
        let mut p = Perfis::default();
        assert!(menor_atraso_em(&p, "cs2"), "sem escolha, vale a sugestão");
        assert!(!menor_atraso_em(&p, "fivem"));

        p.jogos.insert("fivem".into(), perfil(None));
        escolher_atraso_em(&mut p, "fivem", true);
        assert!(menor_atraso_em(&p, "fivem"));
        assert!(!p.jogos.contains_key("fivem"), "a receita era do Mais FPS: o próximo Turbo mede de novo");

        p.jogos.insert("cs2".into(), perfil(Some(Desfecho::Piorou)));
        escolher_atraso_em(&mut p, "cs2", false);
        assert!(!menor_atraso_em(&p, "cs2"));
        assert!(p.jogos.contains_key("cs2"), "a reprovação fica");

        p.jogos.insert("valorant".into(), perfil(None));
        escolher_atraso_em(&mut p, "valorant", true);
        assert!(p.jogos.contains_key("valorant"), "escolher o que já valia não muda nada");
    }

    #[test]
    fn o_que_faltou_na_medida_aparece_na_receita() {
        let r = montar(Gargalo::Placa, None, None, false);
        assert!(r.explicacao.contains("PresentMon não mediu"));
        assert!(r.explicacao.contains("60 Hz"));
        let r = montar(Gargalo::Placa, Some(40.0), Some(144), false);
        assert!(!r.explicacao.contains("PresentMon") && !r.explicacao.contains("60 Hz"));
    }

    #[test]
    fn os_competitivos_estao_no_catalogo() {
        for id in COMPETITIVOS {
            assert!(
                super::super::jogos::CATALOGO.iter().any(|j| j.id == *id),
                "{} não está no catálogo de jogos",
                id
            );
        }
        assert!(!competitivo("fivem"));
    }

    fn sessao(segundos: f64, jogo: f64, tela: f64) -> crate::modules::windows::geracao::resumo::Resumo {
        crate::modules::windows::geracao::resumo::Resumo {
            segundos,
            fps_do_jogo: jogo,
            fps_na_tela: tela,
            na_tela_medido: true,
        }
    }

    #[test]
    fn resultado_com_escala_mostra_o_ganho_do_jogo() {
        let r = resultado_de(Some(41.0), Some(&sessao(600.0, 58.4, 58.4)), Some(Nivel::Qualidade), false).unwrap();
        assert_eq!(r.fps_antes, 41.0);
        assert_eq!(r.fps_do_jogo, 58.0);
        assert_eq!(r.ganho_pct, 42.4);
        assert_eq!(r.fps_na_tela, None, "sem geração, na tela é o próprio jogo");
        assert_eq!(r.minutos, 10.0);
    }

    #[test]
    fn resultado_com_geracao_mostra_a_tela() {
        let r = resultado_de(Some(45.0), Some(&sessao(120.0, 44.0, 132.0)), None, true).unwrap();
        assert_eq!(r.fps_na_tela, Some(132.0));
        assert!(r.ganho_pct < 0.0, "a geração custa um pouco do jogo, e isso aparece");
    }

    #[test]
    fn sem_antes_ou_sem_sessao_nao_ha_resultado() {
        assert_eq!(resultado_de(None, Some(&sessao(60.0, 50.0, 50.0)), None, false), None);
        assert_eq!(resultado_de(Some(40.0), None, None, false), None);
    }

    #[test]
    fn a_prova_tira_a_geracao_e_fica_com_escala_e_energia() {
        let placa = montar(Gargalo::Placa, Some(40.0), Some(144), false);
        assert_eq!(placa.multiplicador, Some(0), "a receita tem geração");
        let p = receita_da_prova(&placa).unwrap();
        assert_eq!(p.multiplicador, Some(1), "na prova, só escala");
        assert_eq!(p.escala, placa.escala);
        assert!(p.energia);

        let cpu = receita_da_prova(&montar(Gargalo::Processador, Some(45.0), Some(144), false)).unwrap();
        assert_eq!(cpu.multiplicador, None, "processador: a prova é da energia");
        assert!(cpu.energia);

        assert_eq!(receita_da_prova(&montar(Gargalo::Nenhum, None, None, false)), None);
    }

    #[test]
    fn a_descricao_diz_o_que_foi_provado() {
        let r = montar(Gargalo::Placa, Some(110.0), Some(144), false);
        let p = receita_da_prova(&r).unwrap();
        assert_eq!(
            descrever(&p),
            format!("Turbo: escala {} + energia de jogo", p.escala.unwrap().nome())
        );
    }

    #[test]
    fn as_frases_da_prova_falam_do_turbo() {
        assert!(leitura_da_prova(Desfecho::Ganhou, "x").contains("Turbo fica"));
        assert!(leitura_da_prova(Desfecho::Piorou, "x").contains("desligado"));
        assert_eq!(leitura_da_prova(Desfecho::SemComparacaoJusta, "o motivo"), "o motivo");
    }

    fn rodada(lado: crate::modules::provaalternada::Lado, fps: f64) -> crate::modules::provaalternada::Rodada {
        crate::modules::provaalternada::Rodada {
            lado,
            fps_do_jogo: fps,
            low_1pct: Some(fps * 0.7),
            fps_exibido: Some(fps),
            quadros: (fps * 30.0) as usize,
            geracao: Some(crate::modules::medicoes::GeracaoNaPartida::NenhumaVisivel),
            configuracao_do_jogo: None,
            pelo_presentmon: true,
        }
    }

    #[test]
    fn uma_prova_que_ganhou_vira_texto_para_compartilhar() {
        use crate::modules::provaalternada::{decidir, Lado};
        let ruido = [0.4, -0.3, 0.2, -0.5, 0.1, 0.3, -0.2, 0.0];
        let rodadas: Vec<_> = ORDEM
            .iter()
            .zip(ruido)
            .map(|(l, r)| rodada(*l, if *l == Lado::ComOtimiza { 58.0 } else { 41.0 } + r))
            .collect();
        let mut prova = decidir("Cyberpunk2077.exe", 0, RODADA_DA_PROVA_S, rodadas, false);
        prova.leitura = leitura_da_prova(prova.desfecho, &prova.leitura);
        assert_eq!(prova.desfecho, Desfecho::Ganhou);
        let receita = receita_da_prova(&montar(Gargalo::Placa, Some(41.0), Some(144), false)).unwrap();
        let texto = crate::modules::compartilhar::texto_do_turbo(
            Some(&prova),
            &descrever(&receita),
            (Some("i3-10100F"), Some("GTX 1650")),
            "22/10/2026",
        )
        .unwrap();
        assert!(texto.contains("sem o Turbo × Turbo: escala"), "{}", texto);
        assert!(texto.contains("8 rodadas de 30 s"), "{}", texto);
        assert!(texto.contains("ganho no FPS"), "{}", texto);
    }

    #[test]
    fn o_aviso_de_copia_so_vem_com_escala_e_copia_medida() {
        use crate::modules::windows::presentmon::{ApiGrafica, Apresentacao};
        let dx11 = aviso_de_copia(Apresentacao::Copiada, ApiGrafica::DirectX, true).unwrap();
        assert!(dx11.contains("DirectX 12 ou Vulkan"), "{}", dx11);
        assert!(aviso_de_copia(Apresentacao::Copiada, ApiGrafica::Outra, true).is_some());
        assert_eq!(aviso_de_copia(Apresentacao::Copiada, ApiGrafica::DirectX, false), None, "sem escala o jogo não foi para janela");
        assert_eq!(aviso_de_copia(Apresentacao::Composta, ApiGrafica::DirectX, true), None);
        assert_eq!(aviso_de_copia(Apresentacao::Desconhecida, ApiGrafica::DirectX, true), None, "sem medida, sem aviso");
    }

    #[test]
    fn a_prova_dura_uns_cinco_minutos() {
        assert!((240..=360).contains(&duracao_da_prova_s()), "{}", duracao_da_prova_s());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn so_escala_nao_conta_como_gerador_de_quadros() {
        use crate::modules::windows::geracao::{Estado, Situacao};
        let so_escala = Estado {
            situacao: Situacao::Gerando,
            multiplicador: 1,
            ..Default::default()
        };
        assert!(!crate::modules::medicoes::gera_quadros(&so_escala));
        assert!(crate::modules::medicoes::gera_quadros(&Estado {
            multiplicador: 2,
            ..so_escala.clone()
        }));
        assert!(crate::modules::medicoes::gera_quadros(&Estado {
            automatico: true,
            ..so_escala
        }));
    }

    #[test]
    fn o_perfil_vai_e_volta_pelo_arquivo() {
        let pasta = std::env::temp_dir().join(format!("otimiza-turbo-{}", std::process::id()));
        let c = pasta.join("turbo.json");
        let mut p = Perfis::default();
        p.jogos.insert(
            "cs2".to_string(),
            Perfil {
                receita: montar(Gargalo::Placa, Some(90.0), Some(144), true),
                fps_antes: Some(90.0),
                medido_em: 1,
                prova: None,
            },
        );
        gravar_em(&c, &p).unwrap();
        assert_eq!(ler_em(&c), p);
        let _ = fs::remove_dir_all(&pasta);
    }

    #[test]
    fn arquivo_ausente_ou_estragado_nao_tem_perfil() {
        let pasta = std::env::temp_dir().join(format!("otimiza-turbo-ruim-{}", std::process::id()));
        assert_eq!(ler_em(&pasta.join("nao-existe.json")), Perfis::default());
        fs::create_dir_all(&pasta).unwrap();
        fs::write(pasta.join("turbo.json"), "{ estragado").unwrap();
        assert_eq!(ler_em(&pasta.join("turbo.json")), Perfis::default());
        let _ = fs::remove_dir_all(&pasta);
    }
}
