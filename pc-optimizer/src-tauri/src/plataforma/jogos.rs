// O perfil de cada jogo: o que ele aceita e o que ele recusa. O FiveM vira um jogo do catálogo como os outros, e jogo
// fora do catálogo continua funcionando com o perfil genérico (a detecção é pelos sinais medidos em
// `modules::windows::deteccao`, não por esta lista).
//
// Só entra aqui o que é estável e público: executável, loja, anticheat. Se o jogo TEM geração de quadros própria não
// se escreve à mão (muda a cada atualização): sai dos arquivos na pasta do jogo (`geracao_nos_arquivos`).

use serde::Serialize;
use std::path::Path;

use super::compat::Api;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AntiCheatDoJogo {
    Nenhum,
    /// O da própria Cfx.re, no FiveM e no RedM.
    CfxRe,
    Vac,
    EasyAntiCheat,
    BattlEye,
    Vanguard,
    Ricochet,
    Hyperion,
    Desconhecido,
}

impl AntiCheatDoJogo {
    pub fn nome(self) -> &'static str {
        match self {
            AntiCheatDoJogo::Nenhum => "sem anticheat",
            AntiCheatDoJogo::CfxRe => "anticheat da Cfx.re",
            AntiCheatDoJogo::Vac => "Valve Anti-Cheat",
            AntiCheatDoJogo::EasyAntiCheat => "Easy Anti-Cheat",
            AntiCheatDoJogo::BattlEye => "BattlEye",
            AntiCheatDoJogo::Vanguard => "Riot Vanguard",
            AntiCheatDoJogo::Ricochet => "Ricochet",
            AntiCheatDoJogo::Hyperion => "Hyperion",
            AntiCheatDoJogo::Desconhecido => "anticheat não identificado",
        }
    }
}

/// Janela por cima do jogo (mira na tela, gerador do Otimiza). `Barrada` nunca abre; `Desaconselhada` abre só se a
/// pessoa confirmar, com o motivo na tela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Sobreposicao {
    Segura,
    Desaconselhada(&'static str),
    Barrada(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Fov {
    /// O jogo não deixa mudar, ou o servidor manda.
    NaoTem,
    /// No arquivo de configuração do próprio jogo, com ele fechado.
    NoArquivo,
    /// Existe no jogo, mas o Otimiza ainda não sabe onde fica gravado com segurança.
    AindaNaoMapeado,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerfilDoJogo {
    pub id: &'static str,
    pub nome: &'static str,
    /// Em minúsculas. Terminando em `_`, casa pelo começo (`fivem_` pega `fivem_b3258_gtaprocess.exe`).
    pub executaveis: &'static [&'static str],
    pub steam_appid: Option<u32>,
    pub anticheat: AntiCheatDoJogo,
    pub api: Api,
    pub sobreposicao: Sobreposicao,
    /// A mira do próprio jogo dá para gravar por arquivo (CS2: `cl_crosshair*`).
    pub mira_no_jogo: bool,
    pub fov: Fov,
    pub nota: Option<&'static str>,
}

const VANGUARD: &str =
    "O Vanguard vigia qualquer janela por cima do Valorant. Use a mira do próprio jogo.";
const RICOCHET: &str = "O Ricochet é de núcleo e trata sobreposições como suspeitas.";
const SERVIDOR_RP: &str =
    "Alguns servidores de RP proíbem janela por cima do jogo (mira, FPS na tela, Frame Boost): confira as regras do seu.";

pub const CATALOGO: &[PerfilDoJogo] = &[
    PerfilDoJogo {
        id: "fivem",
        nome: "FiveM",
        // O lançador `fivem.exe` não desenha; o processo do jogo muda de nome a cada versão.
        executaveis: &["fivem_"],
        steam_appid: None,
        anticheat: AntiCheatDoJogo::CfxRe,
        api: Api::DirectX11,
        sobreposicao: Sobreposicao::Desaconselhada(SERVIDOR_RP),
        mira_no_jogo: false,
        // `profile_fpsFieldOfView` no fivem.cfg (`fov_fivem`).
        fov: Fov::NoArquivo,
        nota: Some(
            "Sem DLSS nem FSR com geração de quadros: o Frame Boost vem do driver ou do Otimiza.",
        ),
    },
    PerfilDoJogo {
        id: "redm",
        nome: "RedM",
        executaveis: &["redm_"],
        steam_appid: None,
        anticheat: AntiCheatDoJogo::CfxRe,
        api: Api::Vulkan,
        sobreposicao: Sobreposicao::Desaconselhada(SERVIDOR_RP),
        mira_no_jogo: false,
        fov: Fov::AindaNaoMapeado,
        nota: None,
    },
    PerfilDoJogo {
        id: "gta5",
        nome: "GTA V",
        executaveis: &["gta5.exe"],
        steam_appid: Some(271590),
        anticheat: AntiCheatDoJogo::BattlEye,
        api: Api::DirectX11,
        sobreposicao: Sobreposicao::Segura,
        mira_no_jogo: false,
        fov: Fov::AindaNaoMapeado,
        nota: None,
    },
    PerfilDoJogo {
        id: "gta5-enhanced",
        nome: "GTA V Enhanced",
        executaveis: &["gta5_enhanced.exe"],
        steam_appid: Some(3240220),
        anticheat: AntiCheatDoJogo::BattlEye,
        api: Api::DirectX12,
        sobreposicao: Sobreposicao::Segura,
        mira_no_jogo: false,
        fov: Fov::AindaNaoMapeado,
        nota: None,
    },
    PerfilDoJogo {
        id: "cs2",
        nome: "Counter-Strike 2",
        executaveis: &["cs2.exe"],
        steam_appid: Some(730),
        anticheat: AntiCheatDoJogo::Vac,
        api: Api::DirectX11,
        sobreposicao: Sobreposicao::Segura,
        mira_no_jogo: true,
        fov: Fov::NaoTem,
        nota: Some("A mira vai para a configuração do próprio jogo, sem sobreposição."),
    },
    PerfilDoJogo {
        id: "valorant",
        nome: "Valorant",
        executaveis: &["valorant-win64-shipping.exe"],
        steam_appid: None,
        anticheat: AntiCheatDoJogo::Vanguard,
        api: Api::DirectX11,
        sobreposicao: Sobreposicao::Barrada(VANGUARD),
        mira_no_jogo: false,
        fov: Fov::NaoTem,
        nota: Some("Com o Vanguard, o Otimiza só mede e ajusta o Windows e o driver."),
    },
    PerfilDoJogo {
        id: "fortnite",
        nome: "Fortnite",
        executaveis: &["fortniteclient-win64-shipping.exe"],
        steam_appid: None,
        anticheat: AntiCheatDoJogo::EasyAntiCheat,
        api: Api::DirectX12,
        sobreposicao: Sobreposicao::Segura,
        mira_no_jogo: false,
        fov: Fov::NaoTem,
        nota: None,
    },
    PerfilDoJogo {
        id: "cod",
        nome: "Call of Duty",
        executaveis: &["cod.exe", "cod24-cod.exe", "blackops6.exe"],
        steam_appid: Some(1938090),
        anticheat: AntiCheatDoJogo::Ricochet,
        api: Api::DirectX12,
        sobreposicao: Sobreposicao::Barrada(RICOCHET),
        mira_no_jogo: false,
        fov: Fov::NaoTem,
        nota: None,
    },
    PerfilDoJogo {
        id: "apex",
        nome: "Apex Legends",
        executaveis: &["r5apex.exe", "r5apex_dx12.exe"],
        steam_appid: Some(1172470),
        anticheat: AntiCheatDoJogo::EasyAntiCheat,
        api: Api::DirectX12,
        sobreposicao: Sobreposicao::Segura,
        mira_no_jogo: false,
        fov: Fov::NaoTem,
        nota: None,
    },
    PerfilDoJogo {
        id: "roblox",
        nome: "Roblox",
        executaveis: &["robloxplayerbeta.exe"],
        steam_appid: None,
        anticheat: AntiCheatDoJogo::Hyperion,
        api: Api::DirectX11,
        sobreposicao: Sobreposicao::Desaconselhada(
            "O Hyperion do Roblox não documenta o que pensa de sobreposições.",
        ),
        mira_no_jogo: false,
        fov: Fov::NaoTem,
        nota: None,
    },
    PerfilDoJogo {
        id: "minecraft",
        nome: "Minecraft",
        executaveis: &["minecraft.windows.exe"],
        steam_appid: None,
        anticheat: AntiCheatDoJogo::Nenhum,
        api: Api::DirectX12,
        sobreposicao: Sobreposicao::Segura,
        mira_no_jogo: false,
        fov: Fov::NaoTem,
        nota: None,
    },
    PerfilDoJogo {
        id: "rocketleague",
        nome: "Rocket League",
        executaveis: &["rocketleague.exe"],
        steam_appid: None,
        anticheat: AntiCheatDoJogo::EasyAntiCheat,
        api: Api::DirectX11,
        sobreposicao: Sobreposicao::Segura,
        mira_no_jogo: false,
        fov: Fov::NaoTem,
        nota: None,
    },
    PerfilDoJogo {
        id: "cyberpunk",
        nome: "Cyberpunk 2077",
        executaveis: &["cyberpunk2077.exe"],
        steam_appid: Some(1091500),
        anticheat: AntiCheatDoJogo::Nenhum,
        api: Api::DirectX12,
        sobreposicao: Sobreposicao::Segura,
        mira_no_jogo: false,
        fov: Fov::NaoTem,
        nota: None,
    },
    PerfilDoJogo {
        id: "rdr2",
        nome: "Red Dead Redemption 2",
        executaveis: &["rdr2.exe"],
        steam_appid: Some(1174180),
        anticheat: AntiCheatDoJogo::Nenhum,
        api: Api::Vulkan,
        sobreposicao: Sobreposicao::Segura,
        mira_no_jogo: false,
        fov: Fov::NaoTem,
        nota: None,
    },
    PerfilDoJogo {
        id: "forza5",
        nome: "Forza Horizon 5",
        executaveis: &["forzahorizon5.exe"],
        steam_appid: Some(1551360),
        anticheat: AntiCheatDoJogo::Nenhum,
        api: Api::DirectX12,
        sobreposicao: Sobreposicao::Segura,
        mira_no_jogo: false,
        fov: Fov::NaoTem,
        nota: None,
    },
];

/// Pura. Pelo nome do executável, sem caminho.
pub fn identificar(executavel: &str) -> Option<&'static PerfilDoJogo> {
    let nome = executavel
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(executavel)
        .to_lowercase();
    CATALOGO.iter().find(|p| {
        p.executaveis.iter().any(|e| {
            if e.ends_with('_') {
                nome.starts_with(e)
            } else {
                nome == *e
            }
        })
    })
}

/// O jogo fora do catálogo: anticheat e API desconhecidos, e a sobreposição desaconselhada até alguém conferir.
pub fn generico() -> PerfilDoJogo {
    PerfilDoJogo {
        id: "outro",
        nome: "Jogo",
        executaveis: &[],
        steam_appid: None,
        anticheat: AntiCheatDoJogo::Desconhecido,
        api: Api::Desconhecida,
        sobreposicao: Sobreposicao::Desaconselhada(
            "Não conheço o anticheat deste jogo. Se ele tiver um de núcleo, uma janela por cima pode ser mal vista.",
        ),
        mira_no_jogo: false,
        fov: Fov::NaoTem,
        nota: None,
    }
}

/// As bibliotecas de geração de quadros que os jogos trazem na pasta. Nomes públicos dos SDKs: o da NVIDIA
/// (Streamline/DLSS-G), o da AMD (FidelityFX) e o da Intel (XeSS FG).
const BIBLIOTECAS_DE_GERACAO: &[(&str, &str)] = &[
    ("nvngx_dlssg.dll", "DLSS Frame Generation"),
    ("sl.dlss_g.dll", "DLSS Frame Generation"),
    ("amd_fidelityfx_dx12.dll", "FSR Frame Generation"),
    ("ffx_frameinterpolation_x64.dll", "FSR Frame Generation"),
    ("libxess_fg.dll", "XeSS Frame Generation"),
];

/// Pura sobre a lista de nomes: o que a pasta do jogo traz de geração própria.
pub fn geracao_nos_nomes<'a>(arquivos: impl IntoIterator<Item = &'a str>) -> Vec<&'static str> {
    let mut achadas: Vec<&'static str> = arquivos
        .into_iter()
        .filter_map(|a| {
            let a = a.to_lowercase();
            BIBLIOTECAS_DE_GERACAO
                .iter()
                .find(|(dll, _)| a == *dll)
                .map(|(_, nome)| *nome)
        })
        .collect();
    achadas.sort();
    achadas.dedup();
    achadas
}

/// Olha a pasta do executável e uma abaixo (Unreal guarda em `Binaries\Win64`, outros em `bin\x64`). Sem descer mais:
/// pasta de jogo tem dezenas de milhares de arquivos, e isto roda com o jogo aberto.
pub fn geracao_nos_arquivos(executavel: &Path) -> Vec<&'static str> {
    let Some(pasta) = executavel.parent() else {
        return Vec::new();
    };
    let mut nomes: Vec<String> = Vec::new();

    let mut ler = |p: &Path| {
        if let Ok(itens) = std::fs::read_dir(p) {
            for item in itens.flatten().take(4000) {
                nomes.push(item.file_name().to_string_lossy().into_owned());
            }
        }
    };
    ler(pasta);
    if let Some(mae) = pasta.parent() {
        ler(mae);
    }

    geracao_nos_nomes(nomes.iter().map(String::as_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fivem_casa_pelo_processo_do_jogo_e_nao_pelo_lancador() {
        assert_eq!(
            identificar("FiveM_b3258_GTAProcess.exe").unwrap().id,
            "fivem"
        );
        assert_eq!(identificar(r"C:\Users\x\AppData\Local\FiveM\FiveM.app\data\cache\subprocess\FiveM_b3570_GTAProcess.exe").unwrap().id, "fivem");
        assert!(identificar("FiveM.exe").is_none(), "o lançador não desenha");
    }

    #[test]
    fn gta_legado_e_enhanced_sao_jogos_diferentes() {
        assert_eq!(identificar("GTA5.exe").unwrap().id, "gta5");
        assert_eq!(
            identificar("GTA5_Enhanced.exe").unwrap().id,
            "gta5-enhanced"
        );
    }

    #[test]
    fn nome_parecido_nao_casa() {
        assert!(identificar("cs2launcher.exe").is_none());
        assert!(identificar("notepad.exe").is_none());
    }

    #[test]
    fn valorant_e_cod_barram_sobreposicao() {
        for exe in ["VALORANT-Win64-Shipping.exe", "cod.exe"] {
            assert!(
                matches!(
                    identificar(exe).unwrap().sobreposicao,
                    Sobreposicao::Barrada(_)
                ),
                "{}",
                exe
            );
        }
    }

    #[test]
    fn so_o_cs2_grava_a_mira_no_proprio_jogo() {
        let com: Vec<&str> = CATALOGO
            .iter()
            .filter(|p| p.mira_no_jogo)
            .map(|p| p.id)
            .collect();
        assert_eq!(com, vec!["cs2"]);
    }

    #[test]
    fn jogo_desconhecido_nunca_e_seguro_para_sobrepor() {
        assert!(matches!(
            generico().sobreposicao,
            Sobreposicao::Desaconselhada(_)
        ));
    }

    #[test]
    fn catalogo_sem_ids_nem_executaveis_repetidos() {
        let mut ids: Vec<&str> = CATALOGO.iter().map(|p| p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), CATALOGO.len());
        let todos: Vec<&str> = CATALOGO
            .iter()
            .flat_map(|p| p.executaveis.iter().copied())
            .collect();
        for e in &todos {
            assert_eq!(*e, e.to_lowercase(), "executável em minúsculas: {}", e);
            assert_eq!(
                todos.iter().filter(|x| *x == e).count(),
                1,
                "{} em dois jogos",
                e
            );
        }
    }

    #[test]
    fn acha_a_geracao_pelos_arquivos_do_jogo() {
        assert_eq!(
            geracao_nos_nomes([
                "Cyberpunk2077.exe",
                "nvngx_dlssg.dll",
                "amd_fidelityfx_dx12.dll"
            ]),
            vec!["DLSS Frame Generation", "FSR Frame Generation"]
        );
        assert_eq!(
            geracao_nos_nomes(["NVNGX_DLSSG.DLL", "sl.dlss_g.dll"]),
            vec!["DLSS Frame Generation"],
            "duas DLLs do mesmo, um só"
        );
        assert!(
            geracao_nos_nomes(["nvngx_dlss.dll"]).is_empty(),
            "DLSS sem o G é só upscaling"
        );
    }

    #[test]
    fn pasta_inexistente_nao_acha_nada() {
        assert!(geracao_nos_arquivos(Path::new(r"Z:\nao\existe\jogo.exe")).is_empty());
    }
}
