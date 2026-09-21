// O catálogo de programas que o técnico instala na máquina do cliente
//
// O QUE ESTA ABA É, E O QUE ELA NÃO É
//
// É uma lista de programas úteis com um botão que os instala sem sair daqui.
// NÃO é um repositório: o Otimiza não hospeda instalador nenhum, não baixa
// binário de endereço escolhido por nós, e não tem cópia de programa de
// terceiro dentro do instalador.
//
// Quem instala é o WINGET, o gerenciador de pacotes que vem com o Windows. A
// diferença não é detalhe de implementação:
//
//   - O pacote vem da fonte oficial do fabricante, verificada pela Microsoft.
//   - A conta de "de onde veio este .exe" é da Microsoft, e não nossa.
//   - Um "otimizador" que baixa executável de um servidor próprio é
//     indistinguível de um que entrega vírus, e não há como o cliente saber a
//     diferença antes de rodar.
//
// O IDENTIFICADOR DO PACOTE É O PONTO FRÁGIL
//
// Mesma lição das capas de jogo: identificador errado não dá erro, INSTALA
// OUTRO PROGRAMA na máquina do cliente — e aqui o estrago é bem maior que uma
// imagem trocada. Todos os identificadores deste arquivo foram conferidos, um
// por um, contra o repositório público de pacotes do winget antes de entrar.
// Quem for acrescentar um: confira antes.
//
// DETECTAR É UMA PERGUNTA, INSTALAR É OUTRA
//
// Saber o que JÁ ESTÁ instalado não passa pelo winget: sai das três chaves de
// desinstalação do registro, que `windows::conflicts::programas_instalados` já
// lê e que existem em toda máquina. É rápido e funciona mesmo onde o winget
// não existe — que, como se descobriu escrevendo isto, é bem mais comum do que
// parece.
//
// O WINGET PODE NÃO EXISTIR, E JUSTO NESTAS MÁQUINAS
//
// A máquina onde este módulo foi escrito tem o pacote `DesktopAppInstaller`
// instalado e NÃO tem `winget.exe`. Windows "lite", debloat agressivo e Loja
// removida deixam exatamente esse estado — e é o perfil do cliente que mais
// precisa de uma lista de programas básicos.
//
// Por isso a aba não some nem quebra sem winget: ela lista, diz o que está
// instalado, e diz que para INSTALAR daqui falta o App Installer.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Categoria {
    /// Ferramentas de sistema que o técnico usa para diagnosticar.
    Sistema,
    /// O que o Windows precisa para jogo rodar: runtimes.
    Base,
    /// Lojas e lançadores.
    Plataformas,
    /// Medir e vigiar hardware.
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
    /// Uma linha dizendo para que serve. Sem adjetivo de propaganda.
    pub descricao: &'static str,
    /// O identificador no winget. CONFERIDO contra o repositório público.
    pub winget: &'static str,
    pub categoria: Categoria,
    /// Pedaços de nome que identificam o programa já instalado, em minúsculas.
    ///
    /// Comparados contra o que as chaves de desinstalação do registro trazem.
    /// Vários porque o nome muda com a versão e com o idioma.
    pub instalado_como: &'static [&'static str],
}

/// A lista. Nomes e identificadores — nenhum binário, nenhum instalador.
pub const CATALOGO: &[Programa] = &[
    // ---- sistema
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
    // ---- base
    //
    // Estes três não são escolha do cliente: são o que falta quando um jogo
    // "não abre" ou fecha sozinho na abertura. A categoria existe separada
    // para o técnico achar rápido.
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
    // ---- plataformas
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
    // ---- monitoramento
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
    // ---- web e mídia
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

/// Um programa do jeito que a tela precisa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NaLista {
    pub id: String,
    pub nome: String,
    pub descricao: String,
    pub categoria: String,
    /// `None` quando não deu para ler os programas instalados.
    ///
    /// AUSENTE NÃO É "NÃO INSTALADO". Mostrar "instalar" sobre um programa que
    /// pode já estar lá é o tipo de botão que faz o técnico instalar de novo
    /// por cima — e alguns instaladores tratam isso como reparo, outros como
    /// primeira instalação.
    pub instalado: Option<bool>,
}

/// Este programa aparece na lista do registro?
pub fn esta_instalado(programa: &Programa, instalados: &[String]) -> bool {
    instalados.iter().any(|nome| {
        let n = nome.to_lowercase();
        programa.instalado_como.iter().any(|p| n.contains(p))
    })
}

/// Monta a lista para a tela.
///
/// `instalados` é `None` quando a leitura do registro falhou — e aí TODOS saem
/// com estado desconhecido, em vez de saírem como não instalados.
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

    /// Identificador repetido significaria dois botões instalando a mesma
    /// coisa — ou, pior, um deles apontando para o pacote errado.
    #[test]
    fn nao_ha_pacote_repetido() {
        let mut pacotes: Vec<&str> = CATALOGO.iter().map(|p| p.winget).collect();
        let antes = pacotes.len();
        pacotes.sort_unstable();
        pacotes.dedup();

        assert_eq!(pacotes.len(), antes, "pacote repetido");
    }

    /// A forma do identificador do winget é `Fabricante.Pacote`.
    ///
    /// Não prova que o pacote existe — isso foi conferido à mão contra o
    /// repositório público. Prova que ninguém escreveu um nome de programa no
    /// lugar de um identificador, que é o engano fácil.
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

    /// A regra que evita reinstalar por cima do que já está lá.
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

    /// Toda categoria do enum aparece na lista. Uma categoria sem nenhum
    /// programa viraria uma aba vazia na tela.
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
