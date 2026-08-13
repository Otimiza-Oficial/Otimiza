// Anticheat: onde o Otimiza tira a mão
//
// POR QUE ESTE MÓDULO EXISTE ANTES DOS OUTROS
//
// O produto passou a fazer duas coisas que, vistas de fora, são indistinguíveis
// de trapaça: SUSPENDE threads de processos de terceiros, e escreve numa chave
// do registro (IFEO) cujo uso mais conhecido é sequestro de execução.
//
// Enquanto a lista de jogos tinha cinco nomes e três eram GTA, isso quase nunca
// encostava num anticheat. Ao abrir a lista para Valorant, Fortnite, PUBG e
// Rainbow Six, encostou — todos esses carregam anticheat de kernel.
//
// Um cliente banido por causa do Otimiza é o pior resultado possível deste
// produto. Pior do que travar a máquina: travamento se conserta, conta banida
// não volta. Então este módulo existe para o produto RECUSAR trabalho, e a
// recusa aparece na tela com o motivo escrito.
//
// COMO A DETECÇÃO É FEITA
//
// Três evidências, todas somente leitura, todas baratas o bastante para rodar
// no laço de seis segundos:
//
//   processo rodando   — o anticheat está ativo agora
//   serviço no boot    — sobe junto com o Windows, mesmo sem o jogo aberto
//   driver instalado   — o cliente joga aquilo, ainda que não agora
//
// O caso do Vanguard é o que justifica as três: o driver `vgk` sobe no boot com
// `Start=0` e fica vigiando a máquina o dia inteiro, com o Valorant fechado.
// Olhar só para processos abertos daria "nenhum anticheat" numa máquina onde a
// Riot está observando desde que o PC ligou.

use super::registry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntiCheat {
    /// Riot Vanguard. Driver de kernel que sobe no boot — Valorant e, desde
    /// 2024, League of Legends.
    Vanguard,
    /// Fortnite, Apex, Rust, Elden Ring e vários outros.
    EasyAntiCheat,
    /// PUBG, Rainbow Six, Tarkov, DayZ, Arma.
    BattlEye,
    /// Counter-Strike. Modo usuário, mas exige a Steam respondendo durante a
    /// partida — suspender a Steam derruba a sessão.
    Vac,
    FaceIt,
}

impl AntiCheat {
    pub fn nome(self) -> &'static str {
        match self {
            AntiCheat::Vanguard => "Riot Vanguard",
            AntiCheat::EasyAntiCheat => "Easy Anti-Cheat",
            AntiCheat::BattlEye => "BattlEye",
            AntiCheat::Vac => "Valve Anti-Cheat",
            AntiCheat::FaceIt => "FACEIT Anti-Cheat",
        }
    }

    /// Se roda com privilégio de núcleo do sistema.
    ///
    /// Anticheat de kernel enxerga manipulação de processo que nenhum programa
    /// comum enxergaria. É a linha que separa "melhor não" de "nunca".
    pub fn e_de_kernel(self) -> bool {
        match self {
            AntiCheat::Vanguard | AntiCheat::EasyAntiCheat | AntiCheat::BattlEye => true,
            AntiCheat::Vac | AntiCheat::FaceIt => false,
        }
    }
}

/// Como soubemos que ele está aí.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Evidencia {
    ProcessoRodando(String),
    /// Serviço configurado para subir com o Windows. O número é o valor de
    /// `Start`: 0 é boot, 1 é sistema, 2 é automático.
    ServicoNoBoot(String, u32),
    DriverInstalado(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presenca {
    pub qual: AntiCheat,
    pub evidencia: Evidencia,
    /// O anticheat está em execução NESTE momento.
    pub ativo_agora: bool,
}

/// Processos de anticheat, em minúsculas.
const PROCESSOS: &[(&str, AntiCheat)] = &[
    ("vgc.exe", AntiCheat::Vanguard),
    ("vgtray.exe", AntiCheat::Vanguard),
    ("easyanticheat.exe", AntiCheat::EasyAntiCheat),
    ("easyanticheat_eos.exe", AntiCheat::EasyAntiCheat),
    ("beservice.exe", AntiCheat::BattlEye),
    ("bedaisy.exe", AntiCheat::BattlEye),
    ("faceitclient.exe", AntiCheat::FaceIt),
    ("faceitservice.exe", AntiCheat::FaceIt),
];

/// Serviços, pelo nome da chave em `CurrentControlSet\Services`.
const SERVICOS: &[(&str, AntiCheat)] = &[
    ("vgc", AntiCheat::Vanguard),
    ("vgk", AntiCheat::Vanguard),
    ("EasyAntiCheat", AntiCheat::EasyAntiCheat),
    ("EasyAntiCheat_EOS", AntiCheat::EasyAntiCheat),
    ("BEService", AntiCheat::BattlEye),
    ("FACEIT", AntiCheat::FaceIt),
];

/// O que o Otimiza quer fazer, para consultar se pode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acao {
    /// Suspender threads de programas de segundo plano.
    SuspenderFundo,
    /// Suspender o lançador da loja (Steam, Epic). Separado do resto porque o
    /// anticheat conversa com ele durante a partida.
    SuspenderLancador,
    /// Abrir handle no processo do jogo para mudar a prioridade.
    PrioridadeNoJogo,
    /// Escrever em Image File Execution Options para o executável do jogo.
    EscreverIfeo,
    /// Trocar o plano de energia do Windows.
    PlanoDeEnergia,
    /// Contar quadros por rastreamento de eventos, sem encostar no processo.
    MedirQuadros,
}

/// A resposta: pode, ou não pode e por quê.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permissao {
    Pode,
    /// Em português, para ir direto à tela do cliente.
    Recusado(String),
}

impl Permissao {
    pub fn pode(&self) -> bool {
        matches!(self, Permissao::Pode)
    }

    pub fn motivo(&self) -> Option<&str> {
        match self {
            Permissao::Pode => None,
            Permissao::Recusado(texto) => Some(texto),
        }
    }
}

/// Decide se uma ação é segura, dado o que foi detectado.
///
/// **Função pura**, e é de propósito: é a regra mais importante do produto em
/// matéria de risco ao cliente, e precisa ser testável sem depender de ter um
/// anticheat instalado na máquina de quem desenvolve.
pub fn permite(acao: Acao, presencas: &[Presenca]) -> Permissao {
    let ativo_de_kernel = presencas
        .iter()
        .find(|p| p.ativo_agora && p.qual.e_de_kernel());

    let qualquer_ativo = presencas.iter().find(|p| p.ativo_agora);
    let instalado_de_kernel = presencas.iter().find(|p| p.qual.e_de_kernel());

    match acao {
        // Nunca encostam em processo nenhum. O rastreamento de eventos do
        // Windows lê o que o sistema já publica; o plano de energia é
        // configuração da máquina. Nenhum dos dois é visível como manipulação.
        Acao::PlanoDeEnergia | Acao::MedirQuadros => Permissao::Pode,

        // Suspender thread de processo alheio é primitiva clássica de trapaça.
        // Com anticheat de kernel rodando, não fazemos, ponto.
        Acao::SuspenderFundo => match ativo_de_kernel {
            Some(p) => Permissao::Recusado(format!(
                "Não pausei nenhum programa: o {} está rodando agora. Pausar programas \
                 com um anticheat de núcleo ativo é risco de banimento, e nenhum ganho \
                 de FPS compensa perder a sua conta.",
                p.qual.nome()
            )),
            None => Permissao::Pode,
        },

        // O lançador conversa com o anticheat durante a partida. Suspender a
        // Steam com Counter-Strike aberto derruba a sessão do VAC — e aqui nem
        // precisa ser anticheat de kernel para dar problema.
        Acao::SuspenderLancador => match qualquer_ativo.or(instalado_de_kernel) {
            Some(p) => Permissao::Recusado(format!(
                "Não pausei o lançador da loja: o {} depende dele durante a partida.",
                p.qual.nome()
            )),
            None => Permissao::Pode,
        },

        // Abrir handle no processo do jogo é a coisa mais visível que o produto
        // faz. E o ganho é pequeno: prioridade alta só muda alguma coisa quando
        // há disputa real de processador.
        Acao::PrioridadeNoJogo => match qualquer_ativo {
            Some(p) => Permissao::Recusado(format!(
                "Não mexi na prioridade do jogo: o {} está ativo, e alterar o processo \
                 do jogo com ele rodando é risco desnecessário. O ganho de prioridade \
                 só aparece quando falta processador — e nunca vale uma conta banida.",
                p.qual.nome()
            )),
            None => Permissao::Pode,
        },

        // Escrever em IFEO deixa marca permanente no registro, na mesma chave
        // usada para sequestro de execução. Para jogo com anticheat de kernel
        // não fazemos nem com o jogo fechado.
        Acao::EscreverIfeo => match instalado_de_kernel {
            Some(p) => Permissao::Recusado(format!(
                "Não gravei a prioridade permanente: esta máquina tem {} instalado. \
                 A chave do registro que guarda essa configuração é a mesma usada por \
                 programas que sequestram a execução de outros, e um anticheat de \
                 núcleo tem todo o direito de estranhar.",
                p.qual.nome()
            )),
            None => Permissao::Pode,
        },
    }
}

// ------------------------------------------------------------------- detecção

/// O que está instalado ou rodando nesta máquina.
///
/// Recebe a lista de processos de fora para poder rodar no laço de seis
/// segundos sem varrer os processos duas vezes.
pub fn detectar(processos: &[String]) -> Vec<Presenca> {
    let mut achados: Vec<Presenca> = Vec::new();

    for nome in processos {
        let minusculo = nome.to_lowercase();

        if let Some((_, qual)) = PROCESSOS.iter().find(|(exe, _)| minusculo == *exe) {
            achados.push(Presenca {
                qual: *qual,
                evidencia: Evidencia::ProcessoRodando(nome.clone()),
                ativo_agora: true,
            });
        }
    }

    for (servico, qual) in SERVICOS {
        // `Start` menor que 3 significa que o serviço sobe sozinho: 0 no boot,
        // 1 com o sistema, 2 automático. É assim que o driver do Vanguard fica
        // vigiando a máquina o dia inteiro com o Valorant fechado.
        let caminho = format!(r"SYSTEM\CurrentControlSet\Services\{}", servico);

        let Ok(crate::modules::changelog::PreviousValue::Dword(inicio)) =
            registry::read("HKLM", &caminho, "Start")
        else {
            continue;
        };

        if inicio >= 3 {
            continue;
        }

        // Se o processo já foi encontrado rodando, aquela evidência é mais
        // forte — não duplicamos a mesma família na lista.
        if achados.iter().any(|p| p.qual == *qual && p.ativo_agora) {
            continue;
        }

        achados.push(Presenca {
            qual: *qual,
            evidencia: Evidencia::ServicoNoBoot(servico.to_string(), inicio),
            // Serviço que sobe no boot está de pé agora, mesmo sem o jogo. É o
            // caso do `vgk` — e tratá-lo como inativo seria o erro que este
            // módulo existe para não cometer.
            ativo_agora: inicio == 0,
        });
    }

    achados
}

/// Lê os processos e detecta. Conveniência para quem não tem a lista em mãos.
pub fn detectar_agora() -> Vec<Presenca> {
    let processos: Vec<String> = super::processes::listar_para_suspensao()
        .into_iter()
        .map(|(_, nome, _)| nome)
        .collect();

    detectar(&processos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rodando(qual: AntiCheat) -> Presenca {
        Presenca {
            qual,
            evidencia: Evidencia::ProcessoRodando("teste.exe".to_string()),
            ativo_agora: true,
        }
    }

    fn instalado(qual: AntiCheat) -> Presenca {
        Presenca {
            qual,
            evidencia: Evidencia::DriverInstalado("teste.sys".to_string()),
            ativo_agora: false,
        }
    }

    #[test]
    fn com_anticheat_de_kernel_ativo_nao_suspende_nada() {
        // A regra mais importante deste arquivo. Um cliente banido por causa do
        // Otimiza é pior do que um cliente com o PC travando.
        for qual in [
            AntiCheat::Vanguard,
            AntiCheat::EasyAntiCheat,
            AntiCheat::BattlEye,
        ] {
            let p = permite(Acao::SuspenderFundo, &[rodando(qual)]);
            assert!(!p.pode(), "{:?} rodando e ainda assim suspendeu", qual);
            assert!(p.motivo().unwrap().contains("banimento"));
        }
    }

    #[test]
    fn maquina_limpa_pode_tudo() {
        // Recusar sem motivo seria o outro extremo do erro: o produto tem que
        // entregar o que o cliente comprou quando não há risco.
        for acao in [
            Acao::SuspenderFundo,
            Acao::SuspenderLancador,
            Acao::PrioridadeNoJogo,
            Acao::EscreverIfeo,
            Acao::PlanoDeEnergia,
            Acao::MedirQuadros,
        ] {
            assert!(permite(acao, &[]).pode(), "{:?} recusada sem motivo", acao);
        }
    }

    #[test]
    fn medir_quadros_e_plano_de_energia_nunca_sao_recusados() {
        // Rastreamento de eventos não encosta no processo do jogo, e plano de
        // energia é configuração da máquina. Recusar aqui seria abrir mão de
        // ganho real sem nenhum risco em troca.
        let todos: Vec<Presenca> = [
            AntiCheat::Vanguard,
            AntiCheat::EasyAntiCheat,
            AntiCheat::BattlEye,
            AntiCheat::Vac,
        ]
        .into_iter()
        .map(rodando)
        .collect();

        assert!(permite(Acao::MedirQuadros, &todos).pode());
        assert!(permite(Acao::PlanoDeEnergia, &todos).pode());
    }

    #[test]
    fn o_vac_nao_e_de_kernel_mas_protege_o_lancador() {
        // Suspender a Steam com Counter-Strike aberto derruba a sessão do VAC,
        // mesmo o VAC não sendo anticheat de núcleo.
        let vac = vec![rodando(AntiCheat::Vac)];

        assert!(permite(Acao::SuspenderFundo, &vac).pode());
        assert!(!permite(Acao::SuspenderLancador, &vac).pode());
    }

    #[test]
    fn ifeo_e_recusado_mesmo_com_o_jogo_fechado() {
        // A escrita em IFEO deixa marca permanente no registro. Não adianta
        // esperar o jogo fechar: a marca continua lá quando ele abrir.
        let p = permite(Acao::EscreverIfeo, &[instalado(AntiCheat::Vanguard)]);

        assert!(!p.pode());
        assert!(p.motivo().unwrap().contains("sequestram a execução"));
    }

    #[test]
    fn anticheat_instalado_mas_parado_nao_impede_suspender() {
        // Recusar por instalação seria recusar em quase toda máquina de quem
        // joga — e a suspensão é justamente o que devolve memória ao jogo.
        assert!(permite(Acao::SuspenderFundo, &[instalado(AntiCheat::EasyAntiCheat)]).pode());
    }

    #[test]
    fn reconhece_o_processo_do_vanguard() {
        let achados = detectar(&["vgc.exe".to_string(), "bloco-de-notas.exe".to_string()]);

        assert_eq!(achados.len(), 1);
        assert_eq!(achados[0].qual, AntiCheat::Vanguard);
        assert!(achados[0].ativo_agora);
    }

    #[test]
    fn detecta_esta_maquina() {
        let achados = detectar_agora();

        for p in &achados {
            println!("  {} — {:?} (ativo: {})", p.qual.nome(), p.evidencia, p.ativo_agora);
        }

        if achados.is_empty() {
            println!("  nenhum anticheat detectado nesta máquina");
        }

        // Não dá para exigir achado nem ausência: depende do que a máquina tem
        // instalado. O que dá para exigir é que a detecção não trave.
        assert!(achados.len() < 20, "detecção devolveu lista implausível");
    }
}
