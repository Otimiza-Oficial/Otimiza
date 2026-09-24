// O registro central: tudo o que o Otimiza altera nesta máquina (2.9)
//
// POR QUE ELE EXISTE
//
// A auditoria da 2.9 achou nove ajustes sem efeito, dois botões que aplicavam
// receita fixa por cima de um motor que mede, e um limpador que apagava a fila
// de downloads do Windows. Nenhum deles era segredo: estavam espalhados por
// vinte e poucos módulos, cada um com a própria porta, e ninguém conseguia ver
// a lista inteira de uma vez — nem quem escreveu o produto.
//
// Este arquivo é essa lista. Cada coisa que o produto muda no computador
// aparece aqui com: onde ela mora no código, o que ela altera, o risco, se
// pede reinício e COMO SE DESFAZ. É o que o pedido do dono chama de registro
// central de alterações.
//
// O QUE ELE NÃO É (ainda)
//
// Não é o motor que executa. Cada módulo continua aplicando o que aplica; o
// que este registro garante é que nada mude no computador do cliente sem estar
// escrito aqui, com o desfazer declarado. A trava
// `todo_modulo_que_escreve_esta_no_registro` reprova a compilação quando um
// módulo novo passa a escrever e ninguém o registrou — o mesmo remédio que o
// `ci_coverage.rs` deu para os módulos fora da esteira.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Risco {
    /// Não muda o que a pessoa vê, e volta inteiro.
    Baixo,
    /// Muda comportamento que a pessoa nota, ou depende da máquina.
    Medio,
    /// Pode custar quadro, ou não volta por aqui.
    Alto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Escopo {
    /// Configuração do Windows.
    Windows,
    /// Só o processo ou o arquivo de UM jogo.
    Jogo,
    /// Driver de vídeo.
    Driver,
    /// Apaga arquivo.
    Arquivo,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alteracao {
    pub id: &'static str,
    pub titulo: &'static str,
    /// Arquivo que aplica, sem `.rs`. É o que a trava confere.
    pub modulo: &'static str,
    pub o_que_muda: &'static str,
    pub risco: Risco,
    pub escopo: Escopo,
    pub precisa_reiniciar: bool,
    /// O próprio sistema refaz o que foi apagado (cache). Não é desfazer, mas
    /// é a diferença entre perder um cache e perder um arquivo do cliente.
    pub refaz_sozinho: bool,
    /// Como se desfaz — em palavras, porque é isto que o cliente precisa ler
    /// ANTES de aplicar.
    pub desfazer: &'static str,
}

/// O que o Otimiza altera fora do catálogo. Os itens do catálogo entram por
/// `todas()`, montados a partir dele para não haver duas listas.
pub static FORA_DO_CATALOGO: &[Alteracao] = &[
    Alteracao {
        id: "plano_energia",
        titulo: "Plano de energia OTIMIZA",
        modulo: "planoenergia",
        o_que_muda: "Cria um plano próprio, copiado do Equilibrado, e o ativa. O plano que você usava não é tocado. Disco, Wi-Fi, multimídia e preferência de placa.",
        risco: Risco::Baixo,
        escopo: Escopo::Windows,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "Reativa o plano que estava ativo antes (fica no histórico).",
    },
    Alteracao {
        id: "motor_energia",
        titulo: "Motor de energia adaptativo",
        modulo: "motorenergia_maquina",
        o_que_muda: "Escreve, dentro do plano OTIMIZA, os valores de processador que ele mediu nesta máquina — e só os que ganharam da medição do padrão.",
        risco: Risco::Medio,
        escopo: Escopo::Windows,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "Backup próprio: relê cada valor e devolve, ou volta ao padrão do Windows e apaga o plano.",
    },
    Alteracao {
        id: "nvidia_global",
        titulo: "Ajustes do driver NVIDIA (todos os jogos)",
        modulo: "nvdriver",
        o_que_muda: "Fila de quadros pré-renderizados, filtro de textura e cache de shader no perfil global do driver.",
        risco: Risco::Medio,
        escopo: Escopo::Driver,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "Devolve cada ajuste ao valor de antes, ou ao padrão pela própria NVIDIA (fica no histórico).",
    },
    Alteracao {
        id: "nvidia_perfil_jogo",
        titulo: "Perfil NVIDIA de um jogo",
        modulo: "nvdriver",
        o_que_muda: "Energia e fila de quadros (e filtro de textura na Baixa latência) no perfil do executável do jogo. Não toca V-Sync nem põe limite de FPS.",
        risco: Risco::Medio,
        escopo: Escopo::Driver,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "Apaga o perfil que o Otimiza criou, ou devolve cada ajuste. Também é desfeito sozinho se as próximas partidas medidas piorarem.",
    },
    Alteracao {
        id: "nvidia_limite_fps",
        titulo: "Limite de quadros de um jogo (NVIDIA)",
        modulo: "nvdriver",
        o_que_muda: "O limitador de quadros do driver, no perfil daquele executável. Só quando a pessoa escolhe o número: o Otimiza nunca põe limite sozinho.",
        risco: Risco::Medio,
        escopo: Escopo::Driver,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "Apaga o perfil criado, ou devolve o limite que havia (fica no histórico).",
    },
    Alteracao {
        id: "config_jogo_unreal",
        titulo: "Configuração gráfica de jogo em Unreal Engine",
        modulo: "unreal",
        o_que_muda: "Sombras, pós-processamento, efeitos e folhagem no arquivo de configuração do jogo. Nunca sobe qualidade, nunca mexe em resolução, distância de visão ou limite de FPS.",
        risco: Risco::Medio,
        escopo: Escopo::Jogo,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "O arquivo INTEIRO fica guardado e volta byte a byte. Também é desfeito sozinho se as próximas partidas medidas piorarem.",
    },
    Alteracao {
        id: "config_jogo_fivem",
        titulo: "Configuração gráfica do FiveM/GTA V",
        modulo: "configjogo",
        o_que_muda: "Os ajustes caros do arquivo de configuração do jogo, com prévia antes.",
        risco: Risco::Medio,
        escopo: Escopo::Jogo,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "O arquivo inteiro fica guardado e volta byte a byte.",
    },
    Alteracao {
        id: "monitor_hz",
        titulo: "Taxa de atualização do monitor",
        modulo: "display",
        o_que_muda: "Põe o monitor na maior taxa que ele aceita, quando está abaixo dela.",
        risco: Risco::Baixo,
        escopo: Escopo::Windows,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "Volta à taxa anterior (fica no histórico).",
    },
    Alteracao {
        id: "auto_cpu_set",
        titulo: "Auto CPU Set (núcleos do jogo)",
        modulo: "cpuset",
        o_que_muda: "Em processador híbrido, prende o processo do jogo nos núcleos de desempenho — e só depois de medir que rende.",
        risco: Risco::Medio,
        escopo: Escopo::Jogo,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "\"Esquecer\", ou fechar o jogo: afinidade morre com o processo. Com anticheat rodando, o Otimiza nem mexe.",
    },
    Alteracao {
        id: "modo_jogo",
        titulo: "Modo jogo (programas de fundo)",
        modulo: "governador",
        o_que_muda: "Enquanto o jogo roda, programas de fundo que estão disputando processador passam para prioridade baixa e modo econômico. Nada é fechado nem congelado. Só age com a medição automática ligada: partidas com e sem ele são comparadas, e se o FPS médio ou o 1% pior caírem ele para sozinho naquele jogo.",
        risco: Risco::Baixo,
        escopo: Escopo::Windows,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "Volta sozinho quando o jogo fecha, quando a comparação mostra queda de FPS, e na abertura seguinte do Otimiza. O que o Windows recusar devolver fica anotado e é tentado de novo.",
    },
    Alteracao {
        id: "servicos_terceiros",
        titulo: "Serviços de outros programas",
        modulo: "servicesaudit",
        o_que_muda: "Passa serviço de terceiro de \"automático\" para \"manual\". Serviço do Windows não entra.",
        risco: Risco::Medio,
        escopo: Escopo::Windows,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "Devolve o tipo de inicialização anterior (fica no histórico).",
    },
    Alteracao {
        id: "tarefas_agendadas",
        titulo: "Tarefas agendadas",
        modulo: "tasks",
        o_que_muda: "Desliga tarefa agendada de atualizador de terceiro.",
        risco: Risco::Medio,
        escopo: Escopo::Windows,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "Liga a tarefa de novo (fica no histórico).",
    },
    Alteracao {
        id: "inicializacao",
        titulo: "Programas que abrem com o Windows",
        modulo: "startup",
        o_que_muda: "Liga ou desliga uma entrada de inicialização, do mesmo jeito que o Gerenciador de Tarefas.",
        risco: Risco::Baixo,
        escopo: Escopo::Windows,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "Devolve o estado anterior, byte a byte (fica no histórico).",
    },
    Alteracao {
        id: "bloatware",
        titulo: "Aplicativos da Loja que vieram junto",
        modulo: "bloatware",
        o_que_muda: "Remove o aplicativo da Loja que a pessoa escolher.",
        risco: Risco::Alto,
        escopo: Escopo::Windows,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "NÃO pelo Otimiza: é reinstalar pela Microsoft Store.",
    },
    Alteracao {
        id: "paginacao",
        titulo: "Arquivo de paginação",
        modulo: "memory",
        o_que_muda: "Devolve o arquivo de paginação ao \"gerenciado pelo Windows\", quando alguém o fixou num tamanho pequeno.",
        risco: Risco::Medio,
        escopo: Escopo::Windows,
        precisa_reiniciar: true,
        refaz_sozinho: false,
        desfazer: "Refazer a configuração manual no Windows: o valor fixo anterior era escolha de quem o pôs.",
    },
    Alteracao {
        id: "limpeza",
        titulo: "Limpeza (temporários, caches, Lixeira)",
        modulo: "limpar",
        o_que_muda: "Apaga arquivo de pasta de temporário, cache e Lixeira — só o que está marcado, com o tamanho medido antes.",
        risco: Risco::Alto,
        escopo: Escopo::Arquivo,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "NÃO TEM. Arquivo apagado não volta, e é por isso que a lista diz o que se perde em cada item e a Lixeira vem desmarcada.",
    },
    Alteracao {
        id: "limpeza_navegador",
        titulo: "Cache dos navegadores",
        modulo: "browsers",
        o_que_muda: "Apaga o cache do navegador escolhido. Histórico, senha e sessão não são tocados.",
        risco: Risco::Medio,
        escopo: Escopo::Arquivo,
        precisa_reiniciar: false,
        refaz_sozinho: true,
        desfazer: "NÃO TEM: o navegador refaz o cache navegando.",
    },
    Alteracao {
        id: "cache_shader",
        titulo: "Cache de shader",
        modulo: "shaders",
        o_que_muda: "Apaga o cache de shader do driver de vídeo, que fica obsoleto quando o driver muda.",
        risco: Risco::Medio,
        escopo: Escopo::Arquivo,
        precisa_reiniciar: false,
        refaz_sozinho: true,
        desfazer: "NÃO TEM: os jogos recompilam os shaders na primeira partida depois da limpeza, que fica mais lenta.",
    },
    Alteracao {
        id: "cache_fivem",
        titulo: "Cache do FiveM",
        modulo: "fivem",
        o_que_muda: "Apaga o cache de servidores do FiveM.",
        risco: Risco::Medio,
        escopo: Escopo::Arquivo,
        precisa_reiniciar: false,
        refaz_sozinho: true,
        desfazer: "NÃO TEM: o FiveM baixa de novo ao entrar no servidor.",
    },
    Alteracao {
        id: "apagar_pasta",
        titulo: "Apagar pasta escolhida no mapa do disco",
        modulo: "foldermap",
        o_que_muda: "Apaga a pasta que a pessoa escolheu no \"Cadê o meu disco?\".",
        risco: Risco::Alto,
        escopo: Escopo::Arquivo,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "NÃO TEM. Por isso o mapa só mostra e a escolha é sempre da pessoa.",
    },
    Alteracao {
        id: "ponto_de_restauracao",
        titulo: "Ponto de restauração",
        modulo: "restore",
        o_que_muda: "Pede ao Windows um ponto de restauração antes de um lote.",
        risco: Risco::Baixo,
        escopo: Escopo::Windows,
        precisa_reiniciar: false,
        refaz_sozinho: false,
        desfazer: "Não precisa: só acrescenta um ponto de volta.",
    },
];

/// Módulos que escrevem, mas NÃO no computador do cliente — dados do próprio
/// Otimiza, arquivo temporário, ou desfazer do que versões antigas deixaram.
/// Cada um com o motivo, para a trava não virar uma lista que ninguém lê.
pub static NAO_ALTERAM_O_WINDOWS: &[(&str, &str)] = &[
    ("afinidade", "põe o processo do jogo em alguns núcleos; a alteração está registrada como auto_cpu_set, que é o único caminho que a usa com medição"),
    ("suspend", "só RETOMA programa que versão antiga suspendeu; o produto não congela mais nada"),
    ("governador", "muda prioridade em memória e volta quando o jogo fecha (registrado como modo jogo)"),
    ("jogos", "cache da biblioteca de jogos, em dados do Otimiza"),
    ("capas", "capas dos jogos, em dados do Otimiza"),
    ("pressao", "janela de pressão de memória, em dados do Otimiza"),
    ("tarefa_longa", "marca de tarefa longa em andamento, em dados do Otimiza"),
    ("citizenfx", "escreve só em arquivo temporário do próprio Otimiza (tem trava própria)"),
    ("geracao", "grava a imagem capturada do laboratório de geração de quadros"),
    ("cpuset", "o arquivo é a escolha guardada; a alteração no Windows está registrada como auto_cpu_set"),
    ("motorenergia_maquina", "o arquivo é o backup do plano; a alteração está registrada como motor_energia"),
    ("limpar", "o arquivo escrito é o registro da própria limpeza, já registrada"),
    ("diskspace", "apaga pelas mesmas pastas do limpar, já registrado"),
    ("power", "biblioteca do powercfg: quem altera são o plano e o catálogo"),
    ("mod", "o motor do catálogo: cada item aplicado está no catálogo"),
    ("devices", "MSI da placa e economia da placa de rede são itens do catálogo"),
    ("gamemode", "item do catálogo (Modo Jogo do Windows)"),
    ("gpupref", "item do catálogo (preferência de placa por programa)"),
    ("network", "item do catálogo (rede)"),
    ("acessibilidade", "item do catálogo (teclas de aderência)"),
];

/// Tudo, com o catálogo junto.
pub fn todas() -> Vec<Alteracao> {
    let mut v: Vec<Alteracao> = FORA_DO_CATALOGO.to_vec();
    for spec in super::catalog::CATALOG.iter().filter(|s| !super::catalog::retirado(s.id)) {
        v.push(Alteracao {
            id: spec.id,
            titulo: spec.name,
            modulo: "catalog",
            o_que_muda: spec.description,
            risco: if spec.security_tradeoff || !spec.reversible {
                Risco::Alto
            } else if spec.risco_de_fps.pode_custar() || matches!(super::catalog::classe(spec.id), super::catalog::Classe::Expert) {
                Risco::Medio
            } else {
                Risco::Baixo
            },
            escopo: Escopo::Windows,
            precisa_reiniciar: spec.requires_restart,
            refaz_sozinho: false,
            desfazer: if spec.reversible {
                "Devolve o valor anterior, guardado no histórico."
            } else {
                "NÃO TEM: apaga arquivo."
            },
        });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Os arquivos de `modules/windows` que chamam uma escrita.
    fn quem_escreve() -> Vec<String> {
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/modules/windows");
        let mut fora = Vec::new();
        let mut pilha = vec![raiz];
        while let Some(dir) = pilha.pop() {
            let Ok(entradas) = std::fs::read_dir(&dir) else { continue };
            for e in entradas.flatten() {
                let caminho = e.path();
                if caminho.is_dir() {
                    pilha.push(caminho);
                    continue;
                }
                if caminho.extension().and_then(|x| x.to_str()) != Some("rs") {
                    continue;
                }
                let Ok(fonte) = std::fs::read_to_string(&caminho) else { continue };
                // Só a parte de fora dos testes: teste que grava num diretório
                // temporário não altera o computador de ninguém.
                let producao = fonte.split("#[cfg(test)]").next().unwrap_or("").to_string();
                let escreve = [
                    "registry::set_",
                    "registry::delete_value",
                    "services::set_start_type",
                    "services::stop(",
                    "shell::powershell_checked",
                    "fs::write",
                    "fs::remove_file",
                    "fs::remove_dir",
                    "SetProcessAffinityMask",
                    "escrever_valor_cru",
                    "ChangeDisplaySettings",
                ]
                .iter()
                .any(|p| producao.contains(p));
                if escreve {
                    let nome = caminho.file_stem().unwrap().to_string_lossy().to_string();
                    let nome = if nome == "mod" {
                        caminho.parent().and_then(|p| p.file_name()).map(|p| p.to_string_lossy().to_string()).unwrap_or(nome)
                    } else {
                        nome
                    };
                    fora.push(nome);
                }
            }
        }
        fora
    }

    /// A TRAVA DESTE ARQUIVO.
    ///
    /// Módulo novo que passa a escrever precisa aparecer no registro — ou na
    /// lista dos que não alteram o Windows, com o motivo. Sem isto, a lista do
    /// que o produto muda envelhece em silêncio, que foi exatamente o que a
    /// auditoria da 2.9 encontrou.
    #[test]
    fn todo_modulo_que_escreve_esta_no_registro() {
        let registrados: Vec<&str> = FORA_DO_CATALOGO
            .iter()
            .map(|a| a.modulo)
            .chain(NAO_ALTERAM_O_WINDOWS.iter().map(|(m, _)| *m))
            // `windows/mod.rs` é o motor do catálogo.
            .chain(["windows", "catalog"])
            .collect();

        let faltando: Vec<String> = quem_escreve()
            .into_iter()
            .filter(|m| !registrados.contains(&m.as_str()))
            .collect();

        assert!(
            faltando.is_empty(),
            "estes módulos escrevem e não estão no registro central: {faltando:?}.\n\
             Acrescente cada um em `FORA_DO_CATALOGO` (com risco e desfazer) ou em \
             `NAO_ALTERAM_O_WINDOWS` (com o motivo)."
        );
    }

    #[test]
    fn todo_item_diz_como_se_desfaz() {
        for a in todas() {
            assert!(a.desfazer.len() >= 20, "{}: desfazer curto demais", a.id);
            assert!(!a.o_que_muda.trim().is_empty(), "{}: sem o que muda", a.id);
        }
    }

    #[test]
    fn o_que_nao_volta_e_sempre_risco_alto() {
        for a in todas()
            .iter()
            .filter(|a| !a.refaz_sozinho)
            .filter(|a| a.desfazer.contains("NÃO TEM") || a.desfazer.contains("NÃO pelo Otimiza"))
        {
            assert_eq!(a.risco, Risco::Alto, "{}: sem desfazer e não está como risco alto", a.id);
        }
    }

    #[test]
    fn nao_ha_id_repetido() {
        let mut ids: Vec<&str> = todas().iter().map(|a| a.id).collect();
        ids.sort();
        let antes = ids.len();
        ids.dedup();
        assert_eq!(antes, ids.len(), "id repetido no registro");
    }
}

#[cfg(test)]
mod nesta_maquina {
    #[test]
    #[ignore]
    fn o_registro_inteiro() {
        for a in super::todas() {
            println!("{:<34} {:?}/{:?}{} — {}", a.id, a.risco, a.escopo, if a.precisa_reiniciar { " (reinício)" } else { "" }, a.desfazer);
        }
    }
}
