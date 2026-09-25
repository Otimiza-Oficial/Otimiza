// Programas úteis instalados pelo WINGET, da fonte oficial de cada fabricante: o Otimiza não hospeda nem baixa
// instalador de endereço próprio. Identificador errado INSTALA OUTRO PROGRAMA: todos foram conferidos contra o
// repositório público do winget, e quem acrescentar confere antes. O que está instalado sai do registro, e a aba
// funciona mesmo sem winget (comum em Windows "lite"), só sem instalar.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Categoria {
    Sistema,
    Base,
    Plataformas,
    Monitoramento,
    Web,
    Midia,
}

impl Categoria {
    pub fn nome(self) -> &'static str {
        match self {
            Categoria::Sistema => "Sistema",
            Categoria::Base => "Base",
            Categoria::Plataformas => "Plataformas",
            Categoria::Monitoramento => "Monitoramento",
            Categoria::Web => "Navegador",
            Categoria::Midia => "Mídia",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Programa {
    pub id: &'static str,
    pub nome: &'static str,
    /// Sem adjetivo de propaganda.
    pub descricao: &'static str,
    /// CONFERIDO contra o repositório público.
    pub winget: &'static str,
    pub categoria: Categoria,
    /// Vários porque o nome muda com a versão e com o idioma.
    pub instalado_como: &'static [&'static str],
}

pub const CATALOGO: &[Programa] = &[
    Programa {
        id: "7zip",
        nome: "7-Zip",
        descricao: "Abre e cria arquivos compactados.",
        winget: "7zip.7zip",
        categoria: Categoria::Sistema,
        instalado_como: &["7-zip"],
    },
    Programa {
        id: "winrar",
        nome: "WinRAR",
        descricao: "Abre .rar. Pago depois do período de avaliação.",
        winget: "RARLab.WinRAR",
        categoria: Categoria::Sistema,
        instalado_como: &["winrar"],
    },
    Programa {
        id: "everything",
        nome: "Everything",
        descricao: "Acha qualquer arquivo do disco na hora, pelo nome.",
        winget: "voidtools.Everything",
        categoria: Categoria::Sistema,
        instalado_como: &["everything"],
    },
    Programa {
        id: "notepadpp",
        nome: "Notepad++",
        descricao: "Editor de texto para mexer em arquivo de configuração.",
        winget: "Notepad++.Notepad++",
        categoria: Categoria::Sistema,
        instalado_como: &["notepad++"],
    },
    Programa {
        id: "procexp",
        nome: "Process Explorer",
        descricao: "Gerenciador de tarefas da Microsoft, com o que o do Windows esconde.",
        winget: "Microsoft.Sysinternals.ProcessExplorer",
        categoria: Categoria::Sistema,
        instalado_como: &["process explorer"],
    },
    Programa {
        id: "autoruns",
        nome: "Autoruns",
        descricao: "Mostra tudo que o Windows abre sozinho — bem além da aba Inicializar.",
        winget: "Microsoft.Sysinternals.Autoruns",
        categoria: Categoria::Sistema,
        instalado_como: &["autoruns"],
    },
    Programa {
        id: "geek",
        nome: "Geek Uninstaller",
        descricao: "Desinstala e varre o que o desinstalador do programa deixou para trás.",
        winget: "GeekUninstaller.GeekUninstaller",
        categoria: Categoria::Sistema,
        instalado_como: &["geek uninstaller"],
    },
    Programa {
        id: "openshell",
        nome: "Open-Shell",
        descricao: "Devolve o menu Iniciar clássico.",
        winget: "Open-Shell.Open-Shell-Menu",
        categoria: Categoria::Sistema,
        instalado_como: &["open-shell", "open shell"],
    },
    // O que falta quando um jogo "não abre" ou fecha na abertura.
    Programa {
        id: "vcredist",
        nome: "Visual C++ 2015-2022 (x64)",
        descricao: "Runtime que a maior parte dos jogos exige. Faltando, o jogo fecha na abertura.",
        winget: "Microsoft.VCRedist.2015+.x64",
        categoria: Categoria::Base,
        instalado_como: &[
            "visual c++ 2015",
            "visual c++ 2017",
            "visual c++ 2019",
            "visual c++ 2022",
        ],
    },
    Programa {
        id: "dotnet8",
        nome: ".NET Desktop Runtime 8",
        descricao: "Runtime exigido por programas e alguns jogos mais novos.",
        winget: "Microsoft.DotNet.DesktopRuntime.8",
        categoria: Categoria::Base,
        instalado_como: &["windows desktop runtime - 8"],
    },
    Programa {
        id: "java",
        nome: "Java Runtime",
        descricao: "Necessário para Minecraft Java e para alguns utilitários.",
        winget: "Oracle.JavaRuntimeEnvironment",
        categoria: Categoria::Base,
        instalado_como: &["java 8", "java(tm)", "java se"],
    },
    Programa {
        id: "steam",
        nome: "Steam",
        descricao: "Loja e biblioteca da Valve.",
        winget: "Valve.Steam",
        categoria: Categoria::Plataformas,
        instalado_como: &["steam"],
    },
    Programa {
        id: "epic",
        nome: "Epic Games Launcher",
        descricao: "Loja e biblioteca da Epic.",
        winget: "EpicGames.EpicGamesLauncher",
        categoria: Categoria::Plataformas,
        instalado_como: &["epic games launcher"],
    },
    Programa {
        id: "discord",
        nome: "Discord",
        descricao: "Voz e texto. A sobreposição dele custa quadros em máquina fraca.",
        winget: "Discord.Discord",
        categoria: Categoria::Plataformas,
        instalado_como: &["discord"],
    },
    Programa {
        id: "hwinfo",
        nome: "HWiNFO",
        descricao: "Lê os sensores que o Windows não publica: temperatura, clock e potência.",
        winget: "REALiX.HWiNFO",
        categoria: Categoria::Monitoramento,
        instalado_como: &["hwinfo"],
    },
    Programa {
        id: "crystaldiskinfo",
        nome: "CrystalDiskInfo",
        descricao: "Saúde do disco pelo S.M.A.R.T. É onde se vê um SSD morrendo.",
        winget: "CrystalDewWorld.CrystalDiskInfo",
        categoria: Categoria::Monitoramento,
        instalado_como: &["crystaldiskinfo"],
    },
    Programa {
        id: "gpuz",
        nome: "GPU-Z",
        descricao: "Identifica a placa de vídeo e mostra o que o driver expõe dela.",
        winget: "TechPowerUp.GPU-Z",
        categoria: Categoria::Monitoramento,
        instalado_como: &["gpu-z"],
    },
    Programa {
        id: "afterburner",
        nome: "MSI Afterburner",
        descricao: "Curva de ventoinha e limite de potência da placa. Mexer aqui exige cuidado.",
        winget: "Guru3D.Afterburner",
        categoria: Categoria::Monitoramento,
        instalado_como: &["msi afterburner"],
    },
    Programa {
        id: "chrome",
        nome: "Google Chrome",
        descricao: "Navegador.",
        winget: "Google.Chrome",
        categoria: Categoria::Web,
        instalado_como: &["google chrome"],
    },
    Programa {
        id: "brave",
        nome: "Brave",
        descricao: "Navegador com bloqueio de anúncio embutido.",
        winget: "Brave.Brave",
        categoria: Categoria::Web,
        instalado_como: &["brave"],
    },
    Programa {
        id: "vlc",
        nome: "VLC",
        descricao: "Toca praticamente qualquer vídeo, sem pacote de codec.",
        winget: "VideoLAN.VLC",
        categoria: Categoria::Midia,
        instalado_como: &["vlc media player"],
    },
    Programa {
        id: "obs",
        nome: "OBS Studio",
        descricao: "Grava e transmite. Gravando, ele mesmo custa quadros.",
        winget: "OBSProject.OBSStudio",
        categoria: Categoria::Midia,
        instalado_como: &["obs studio"],
    },
];

pub fn por_id(id: &str) -> Option<&'static Programa> {
    CATALOGO.iter().find(|p| p.id == id)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NaLista {
    pub id: String,
    pub nome: String,
    pub descricao: String,
    pub categoria: String,
    /// `None`: AUSENTE NÃO É "NÃO INSTALADO"; o botão "instalar" faria o técnico instalar de novo por cima.
    pub instalado: Option<bool>,
}

pub fn esta_instalado(programa: &Programa, instalados: &[String]) -> bool {
    instalados.iter().any(|nome| {
        let n = nome.to_lowercase();
        programa.instalado_como.iter().any(|p| n.contains(p))
    })
}

pub fn montar(instalados: Option<&[String]>) -> Vec<NaLista> {
    CATALOGO
        .iter()
        .map(|p| NaLista {
            id: p.id.to_string(),
            nome: p.nome.to_string(),
            descricao: p.descricao.to_string(),
            categoria: p.categoria.nome().to_string(),
            instalado: instalados.map(|lista| esta_instalado(p, lista)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nao_ha_id_repetido() {
        let mut ids: Vec<&str> = CATALOGO.iter().map(|p| p.id).collect();
        let antes = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), antes, "id repetido");
    }

    #[test]
    fn nao_ha_pacote_repetido() {
        let mut pacotes: Vec<&str> = CATALOGO.iter().map(|p| p.winget).collect();
        let antes = pacotes.len();
        pacotes.sort_unstable();
        pacotes.dedup();

        assert_eq!(pacotes.len(), antes, "pacote repetido");
    }

    /// Não prova que o pacote existe (conferido à mão): prova que ninguém escreveu um nome no lugar do identificador.
    #[test]
    fn todo_pacote_tem_forma_de_identificador() {
        for p in CATALOGO {
            assert!(
                p.winget.contains('.') && !p.winget.contains(' '),
                "{} não parece identificador de winget: {}",
                p.id,
                p.winget
            );
        }
    }

    #[test]
    fn todo_programa_sabe_se_reconhecer_instalado() {
        for p in CATALOGO {
            assert!(
                !p.instalado_como.is_empty(),
                "{} não tem como ser reconhecido instalado",
                p.id
            );
            for nome in p.instalado_como {
                assert_eq!(*nome, nome.to_lowercase(), "{} com maiúscula", p.id);
            }
        }
    }

    #[test]
    fn todo_programa_diz_para_que_serve() {
        for p in CATALOGO {
            assert!(!p.descricao.is_empty(), "{} sem descrição", p.id);
            assert!(
                p.descricao.ends_with('.'),
                "{}: a descrição é uma frase",
                p.id
            );
        }
    }

    #[test]
    fn reconhece_o_que_esta_instalado() {
        let instalados = vec![
            "7-Zip 24.09 (x64)".to_string(),
            "Google Chrome".to_string(),
            "Microsoft Visual C++ 2015-2022 Redistributable (x64) - 14.40".to_string(),
        ];

        assert!(esta_instalado(por_id("7zip").unwrap(), &instalados));
        assert!(esta_instalado(por_id("chrome").unwrap(), &instalados));
        assert!(esta_instalado(por_id("vcredist").unwrap(), &instalados));
        assert!(!esta_instalado(por_id("vlc").unwrap(), &instalados));
    }

    #[test]
    fn registro_ilegivel_deixa_todos_em_desconhecido() {
        let lista = montar(None);

        assert_eq!(lista.len(), CATALOGO.len());
        assert!(
            lista.iter().all(|p| p.instalado.is_none()),
            "sem a leitura do registro, ninguém pode ser dado como não instalado"
        );
    }

    #[test]
    fn com_a_leitura_todos_tem_resposta() {
        let lista = montar(Some(&["7-Zip 24.09".to_string()]));

        assert!(lista.iter().all(|p| p.instalado.is_some()));
        assert_eq!(
            lista.iter().find(|p| p.id == "7zip").unwrap().instalado,
            Some(true)
        );
    }

    #[test]
    fn nenhuma_categoria_fica_vazia() {
        for c in [
            Categoria::Sistema,
            Categoria::Base,
            Categoria::Plataformas,
            Categoria::Monitoramento,
            Categoria::Web,
            Categoria::Midia,
        ] {
            assert!(
                CATALOGO.iter().any(|p| p.categoria == c),
                "categoria {:?} sem nenhum programa",
                c
            );
        }
    }
}
