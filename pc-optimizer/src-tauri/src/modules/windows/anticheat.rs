// Anticheat: onde o Otimiza RECUSA trabalho, com o motivo na tela. Cliente banido é pior que PC travado: conta
// não volta. Três evidências, só leitura: processo rodando, serviço no boot e driver instalado. O Vanguard (`vgk`)
// sobe no boot com `Start=0` e vigia o dia inteiro com o Valorant fechado: olhar só processos não o veria.

use super::registry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntiCheat {
    Vanguard,
    EasyAntiCheat,
    BattlEye,
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

    /// Kernel é a linha entre "melhor não" e "nunca".
    pub fn e_de_kernel(self) -> bool {
        match self {
            AntiCheat::Vanguard | AntiCheat::EasyAntiCheat | AntiCheat::BattlEye => true,
            AntiCheat::Vac | AntiCheat::FaceIt => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Evidencia {
    ProcessoRodando(String),
    ServicoNoBoot(String, u32),
    DriverInstalado(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presenca {
    pub qual: AntiCheat,
    pub evidencia: Evidencia,
    pub ativo_agora: bool,
}

const PROCESSOS: &[(&str, AntiCheat)] = &[
    ("vgc.exe", AntiCheat::Vanguard),
    ("vgtray.exe", AntiCheat::Vanguard),
    // O jogo, não o anticheat: o `vgc` e o `vgtray` rodam desde o boot em todo PC com o Valorant instalado. É o jogo aberto
    // que diz que a partida (e a caça a sobreposições) começou.
    ("valorant-win64-shipping.exe", AntiCheat::Vanguard),
    ("easyanticheat.exe", AntiCheat::EasyAntiCheat),
    ("easyanticheat_eos.exe", AntiCheat::EasyAntiCheat),
    ("beservice.exe", AntiCheat::BattlEye),
    ("bedaisy.exe", AntiCheat::BattlEye),
    ("faceitclient.exe", AntiCheat::FaceIt),
    ("faceitservice.exe", AntiCheat::FaceIt),
];

const SERVICOS: &[(&str, AntiCheat)] = &[
    ("vgc", AntiCheat::Vanguard),
    ("vgk", AntiCheat::Vanguard),
    ("EasyAntiCheat", AntiCheat::EasyAntiCheat),
    ("EasyAntiCheat_EOS", AntiCheat::EasyAntiCheat),
    ("BEService", AntiCheat::BattlEye),
    ("FACEIT", AntiCheat::FaceIt),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acao {
    AfinidadeNoJogo,
    EscreverIfeo,
    PlanoDeEnergia,
    MedirQuadros,
    /// Uma janela nossa por cima do jogo: o Frame Boost do Otimiza e a mira na tela. Não toca no processo do jogo,
    /// mas há anticheat que trata qualquer sobreposição como suspeita.
    SobreporJanela,
    /// Saturação, brilho e contraste do monitor pelo driver de vídeo: o mesmo que o painel da NVIDIA, da AMD ou da
    /// Intel faz, sem encostar no jogo.
    CoresDoMonitor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permissao {
    Pode,
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

/// Pura: esta presença é uma partida que caça sobreposições acontecendo agora?
fn barra_sobreposicao(p: &Presenca) -> bool {
    match (&p.qual, &p.evidencia) {
        (AntiCheat::Vanguard, Evidencia::ProcessoRodando(nome)) => nome.eq_ignore_ascii_case("valorant-win64-shipping.exe"),
        (AntiCheat::FaceIt, Evidencia::ProcessoRodando(_)) => true,
        _ => false,
    }
}

/// Pura: a regra de risco mais importante do produto, testável sem anticheat na máquina de quem desenvolve.
pub fn permite(acao: Acao, presencas: &[Presenca]) -> Permissao {
    let qualquer_ativo = presencas.iter().find(|p| p.ativo_agora);
    let instalado_de_kernel = presencas.iter().find(|p| p.qual.e_de_kernel());

    match acao {
        // Rastreamento de eventos e plano de energia não encostam no processo do jogo.
        Acao::PlanoDeEnergia | Acao::MedirQuadros | Acao::CoresDoMonitor => Permissao::Pode,

        // Só com uma partida que caça sobreposições ABERTA: o Valorant aberto, ou o cliente do FACEIT. O Vanguard que roda
        // desde o boot (o serviço `vgk`, o `vgc` e o ícone `vgtray`) sozinho não bloqueia: quem tem Valorant instalado
        // também joga FiveM, e até a 1.0.5 o Frame Boost e a mira sumiam nele.
        Acao::SobreporJanela => match presencas.iter().find(|p| barra_sobreposicao(p)) {
            Some(p) => Permissao::Recusado(format!(
                "Não abri a janela por cima do jogo: o {} está aberto e trata sobreposições como suspeitas. Feche-o ou use a mira e a geração de quadros do próprio jogo.",
                p.qual.nome()
            )),
            None => Permissao::Pode,
        },

        // Abrir handle no processo do jogo é a coisa mais visível que o produto faz.
        Acao::AfinidadeNoJogo => match qualquer_ativo {
            Some(p) => Permissao::Recusado(format!(
                "Não mexi nos núcleos do jogo: o {} está ativo, e alterar o processo \
                 do jogo com ele rodando é risco desnecessário. Nenhum ganho de núcleo \
                 vale uma conta banida.",
                p.qual.nome()
            )),
            None => Permissao::Pode,
        },

        // IFEO deixa marca permanente na chave usada para sequestro de execução: com anticheat de kernel, nem com o
        // jogo fechado.
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

/// Recebe a lista de processos de fora, para o laço de 6 s não varrer duas vezes.
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
        let caminho = format!(r"SYSTEM\CurrentControlSet\Services\{}", servico);

        let Ok(crate::modules::changelog::PreviousValue::Dword(inicio)) =
            registry::read("HKLM", &caminho, "Start")
        else {
            continue;
        };

        if inicio >= 3 {
            continue;
        }

        if achados.iter().any(|p| p.qual == *qual && p.ativo_agora) {
            continue;
        }

        achados.push(Presenca {
            qual: *qual,
            evidencia: Evidencia::ServicoNoBoot(servico.to_string(), inicio),
            // Serviço que sobe no boot está de pé agora, mesmo sem o jogo.
            ativo_agora: inicio == 0,
        });
    }

    achados
}

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
    fn maquina_limpa_pode_tudo() {
        // Recusar sem motivo é o outro erro: sem risco, o produto entrega.
        for acao in [
            Acao::AfinidadeNoJogo,
            Acao::EscreverIfeo,
            Acao::PlanoDeEnergia,
            Acao::MedirQuadros,
            Acao::SobreporJanela,
            Acao::CoresDoMonitor,
        ] {
            assert!(permite(acao, &[]).pode(), "{:?} recusada sem motivo", acao);
        }
    }

    #[test]
    fn medir_quadros_e_plano_de_energia_nunca_sao_recusados() {
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
    fn sobreposicao_e_recusada_com_vanguard_ou_faceit_abertos() {
        let valorant = Presenca {
            qual: AntiCheat::Vanguard,
            evidencia: Evidencia::ProcessoRodando("VALORANT-Win64-Shipping.exe".to_string()),
            ativo_agora: true,
        };
        for (qual, presenca) in [(AntiCheat::Vanguard, valorant), (AntiCheat::FaceIt, rodando(AntiCheat::FaceIt))] {
            let p = permite(Acao::SobreporJanela, &[presenca]);
            assert!(!p.pode(), "{:?} aberto deveria barrar a sobreposição", qual);
            assert!(p.motivo().unwrap().contains(qual.nome()));
        }
    }

    #[test]
    fn o_vanguard_do_boot_nao_barra_a_sobreposicao_so_o_valorant_aberto() {
        // O PC do sócio: Valorant instalado, FiveM aberto. O vgc e o vgtray rodam desde o boot.
        let do_boot = detectar(&["vgc.exe".to_string(), "vgtray.exe".to_string(), "FiveM_GTAProcess.exe".to_string()]);
        assert!(!do_boot.is_empty(), "o Vanguard continua sendo visto (para a afinidade e o IFEO)");
        assert!(permite(Acao::SobreporJanela, &do_boot).pode(), "Vanguard sem o Valorant aberto não barra o Frame Boost");

        let com_o_jogo = detectar(&["vgc.exe".to_string(), "VALORANT-Win64-Shipping.exe".to_string()]);
        let p = permite(Acao::SobreporJanela, &com_o_jogo);
        assert!(!p.pode(), "com o Valorant aberto, barra");
        assert!(p.motivo().unwrap().contains("Vanguard"));
    }

    #[test]
    fn so_o_servico_do_vanguard_no_boot_nao_barra_a_sobreposicao() {
        let no_boot = Presenca {
            qual: AntiCheat::Vanguard,
            evidencia: Evidencia::ServicoNoBoot("vgk".to_string(), 0),
            ativo_agora: true,
        };
        assert!(permite(Acao::SobreporJanela, &[no_boot]).pode());
    }

    #[test]
    fn sobreposicao_convive_com_eac_battleye_e_vac() {
        let outros: Vec<Presenca> = [
            AntiCheat::EasyAntiCheat,
            AntiCheat::BattlEye,
            AntiCheat::Vac,
        ]
        .into_iter()
        .map(rodando)
        .collect();
        assert!(permite(Acao::SobreporJanela, &outros).pode());
    }

    #[test]
    fn cores_do_monitor_nunca_sao_recusadas() {
        let todos: Vec<Presenca> = [
            AntiCheat::Vanguard,
            AntiCheat::FaceIt,
            AntiCheat::EasyAntiCheat,
        ]
        .into_iter()
        .map(rodando)
        .collect();
        assert!(permite(Acao::CoresDoMonitor, &todos).pode());
    }

    #[test]
    fn ifeo_e_recusado_mesmo_com_o_jogo_fechado() {
        let p = permite(Acao::EscreverIfeo, &[instalado(AntiCheat::Vanguard)]);

        assert!(!p.pode());
        assert!(p.motivo().unwrap().contains("sequestram a execução"));
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
            println!(
                "  {} — {:?} (ativo: {})",
                p.qual.nome(),
                p.evidencia,
                p.ativo_agora
            );
        }

        if achados.is_empty() {
            println!("  nenhum anticheat detectado nesta máquina");
        }

        assert!(achados.len() < 20, "detecção devolveu lista implausível");
    }
}
