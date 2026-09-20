// O catálogo de jogos conhecidos, e o casamento com o que está instalado
//
// POR QUE UM CATÁLOGO, SE O PRODUTO JÁ DETECTA O QUE ESTÁ INSTALADO
//
// `windows::jogos::varrer` responde "o que existe nesta máquina", e responde
// bem. Mas a tela da biblioteca precisa responder outra coisa: "o que o
// Otimiza sabe otimizar". As duas listas não são a mesma, e a diferença é o
// que o cliente precisa ver.
//
//   - Um jogo instalado que o catálogo não conhece continua aparecendo. Ele é
//     otimizável do mesmo jeito: prioridade, afinidade e preferência de placa
//     não dependem de o Otimiza ter ouvido falar do título.
//   - Um jogo conhecido que NÃO está instalado também aparece, marcado como
//     não instalado. É o que permite a pessoa achar o jogo dela na grade em
//     vez de concluir que o produto não o suporta.
//
// O QUE ESTE MÓDULO NÃO GUARDA
//
// ARTE. Nenhuma capa, nenhum logotipo, nenhum ícone de jogo. Duas razões, e a
// primeira basta: a arte de cada jogo é de quem fez o jogo, e embutir isso no
// instalador do Otimiza é distribuir material de terceiro sem permissão. A
// segunda é prática — seriam dezenas de megabytes de imagem no instalador para
// desenhar uma grade.
//
// No lugar, cada jogo ganha UMA COR derivada do próprio nome e as iniciais. É
// estável (o mesmo nome dá sempre a mesma cor), é distinguível de relance, e
// quem quiser a capa de verdade aponta um arquivo da própria máquina — que é
// o que o campo de capa do lado da tela faz.
//
// COMO O CASAMENTO FUNCIONA
//
// Pelo NOME DO EXECUTÁVEL, e não pelo nome bonito do jogo. O nome que a loja
// mostra muda com tradução, com edição e com ano ("EA SPORTS FC 26", "EA
// SPORTS FC™ 26"); o executável é o que o sistema operacional usa e é o mesmo
// que a otimização por jogo precisa escrever. Quando a loja não informa o
// executável — a Steam não informa —, cai para o nome da pasta, que é a
// segunda coisa mais estável.

use serde::{Deserialize, Serialize};

/// Um jogo que o produto conhece pelo nome.
///
/// SÓ SERIALIZA, não desserializa: o catálogo é constante do programa, e uma
/// entrada vinda de fora significaria alguém escolhendo com que executável o
/// produto vai mexer — que é exatamente a decisão que `jogos::dentro_de_
/// biblioteca` existe para não deixar de fora do nosso controle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct JogoConhecido {
    /// Identificador estável. Nunca muda, nem quando o nome de tela muda.
    pub id: &'static str,
    pub nome: &'static str,
    /// Executáveis que identificam este jogo, em minúsculas e com `.exe`.
    ///
    /// Vários porque um jogo tem lançador e cliente, e porque alguns trocaram
    /// de executável entre versões.
    pub executaveis: &'static [&'static str],
    /// Pedaços de nome de pasta que também identificam, em minúsculas.
    ///
    /// Servem para a Steam, que não informa o executável no manifesto.
    pub pastas: &'static [&'static str],
    /// O número do jogo na Steam, para buscar a capa.
    ///
    /// TODOS OS QUE ESTÃO AQUI FORAM CONFERIDOS contra a API pública da
    /// Steam, um por um. Número errado não dá erro: dá a capa de OUTRO jogo
    /// no bloco, que é pior que letra nenhuma — é o produto afirmando com
    /// confiança uma coisa errada, que é o defeito que ele existe para não
    /// ter. Quem for acrescentar um: confira antes.
    ///
    /// `None` para jogo que não está na Steam. Valorant, Fortnite, League,
    /// Minecraft, Roblox, Tarkov e FiveM não estão, e inventar um número
    /// para eles cairia justamente no caso acima.
    pub appid: Option<u32>,
}

/// Os jogos que o produto reconhece de nome.
///
/// A lista é de NOMES E EXECUTÁVEIS — fatos públicos sobre o que existe numa
/// máquina —, e não de arte nem de conteúdo de ninguém.
///
/// Ela não é um limite: jogo fora desta lista aparece na biblioteca do mesmo
/// jeito assim que for detectado no disco. Estar aqui só serve para o título
/// aparecer ANTES de estar instalado, e para o produto acertar o nome de tela.
pub const CATALOGO: &[JogoConhecido] = &[
    JogoConhecido {
        id: "cs2",
        nome: "Counter-Strike 2",
        executaveis: &["cs2.exe"],
        pastas: &["counter-strike global offensive", "counter-strike 2"],
        appid: Some(730),
    },
    JogoConhecido {
        id: "valorant",
        nome: "Valorant",
        executaveis: &["valorant.exe", "valorant-win64-shipping.exe"],
        pastas: &["riot games/valorant", "valorant"],
        appid: None,
    },
    JogoConhecido {
        id: "fortnite",
        nome: "Fortnite",
        executaveis: &["fortniteclient-win64-shipping.exe", "fortnitelauncher.exe"],
        pastas: &["fortnite"],
        appid: None,
    },
    JogoConhecido {
        id: "warzone",
        nome: "Call of Duty: Warzone",
        executaveis: &["cod.exe", "modernwarfare.exe"],
        pastas: &["call of duty"],
        appid: Some(1962663),
    },
    JogoConhecido {
        id: "apex",
        nome: "Apex Legends",
        executaveis: &["r5apex.exe", "r5apex_dx12.exe"],
        pastas: &["apex legends"],
        appid: Some(1172470),
    },
    JogoConhecido {
        id: "rust",
        nome: "Rust",
        executaveis: &["rustclient.exe"],
        pastas: &["rust"],
        appid: Some(252490),
    },
    JogoConhecido {
        id: "pubg",
        nome: "PUBG: Battlegrounds",
        executaveis: &["tslgame.exe"],
        pastas: &["pubg"],
        appid: Some(578080),
    },
    JogoConhecido {
        id: "gtav",
        nome: "Grand Theft Auto V",
        executaveis: &["gta5.exe", "gtav.exe", "playgtav.exe"],
        pastas: &["grand theft auto v"],
        appid: Some(271590),
    },
    JogoConhecido {
        id: "fivem",
        nome: "FiveM",
        executaveis: &["fivem.exe", "fivem_b2699_gtaprocess.exe"],
        pastas: &["fivem"],
        appid: None,
    },
    JogoConhecido {
        id: "eafc",
        nome: "EA SPORTS FC",
        executaveis: &["fc25.exe", "fc26.exe", "fc24.exe"],
        pastas: &["ea sports fc"],
        appid: Some(2669320),
    },
    JogoConhecido {
        id: "lol",
        nome: "League of Legends",
        executaveis: &["league of legends.exe", "leagueclient.exe"],
        pastas: &["league of legends"],
        appid: None,
    },
    JogoConhecido {
        id: "dota2",
        nome: "Dota 2",
        executaveis: &["dota2.exe"],
        pastas: &["dota 2"],
        appid: Some(570),
    },
    JogoConhecido {
        id: "r6",
        nome: "Rainbow Six Siege",
        executaveis: &["rainbowsix.exe", "rainbowsix_vulkan.exe"],
        pastas: &["tom clancy's rainbow six siege"],
        appid: Some(359550),
    },
    JogoConhecido {
        id: "overwatch2",
        nome: "Overwatch 2",
        executaveis: &["overwatch.exe"],
        pastas: &["overwatch"],
        appid: Some(2357570),
    },
    JogoConhecido {
        id: "minecraft",
        nome: "Minecraft",
        executaveis: &["minecraft.windows.exe", "javaw.exe"],
        pastas: &["minecraft"],
        appid: None,
    },
    JogoConhecido {
        id: "roblox",
        nome: "Roblox",
        executaveis: &["robloxplayerbeta.exe"],
        pastas: &["roblox"],
        appid: None,
    },
    JogoConhecido {
        id: "tarkov",
        nome: "Escape from Tarkov",
        executaveis: &["escapefromtarkov.exe"],
        pastas: &["escape from tarkov"],
        appid: None,
    },
    JogoConhecido {
        id: "thefinals",
        nome: "The Finals",
        executaveis: &["discovery.exe"],
        pastas: &["the finals"],
        appid: Some(2073850),
    },
    JogoConhecido {
        id: "marvelrivals",
        nome: "Marvel Rivals",
        executaveis: &["marvel-win64-shipping.exe"],
        pastas: &["marvelrivals", "marvel rivals"],
        appid: Some(2767030),
    },
    JogoConhecido {
        id: "deltaforce",
        nome: "Delta Force",
        executaveis: &["deltaforceclient-win64-shipping.exe"],
        pastas: &["delta force"],
        appid: Some(2507950),
    },
    JogoConhecido {
        id: "valheim",
        nome: "Valheim",
        executaveis: &["valheim.exe"],
        pastas: &["valheim"],
        appid: Some(892970),
    },
    JogoConhecido {
        id: "elden-ring",
        nome: "Elden Ring",
        executaveis: &["eldenring.exe"],
        pastas: &["elden ring"],
        appid: Some(1245620),
    },
    JogoConhecido {
        id: "cyberpunk",
        nome: "Cyberpunk 2077",
        executaveis: &["cyberpunk2077.exe"],
        pastas: &["cyberpunk 2077"],
        appid: Some(1091500),
    },
    JogoConhecido {
        id: "helldivers2",
        nome: "Helldivers 2",
        executaveis: &["helldivers2.exe"],
        pastas: &["helldivers 2"],
        appid: Some(553850),
    },
    JogoConhecido {
        id: "rocketleague",
        nome: "Rocket League",
        executaveis: &["rocketleague.exe"],
        pastas: &["rocketleague", "rocket league"],
        appid: Some(252950),
    },
];

/// Um jogo do jeito que a grade da biblioteca precisa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NaGrade {
    pub id: String,
    pub nome: String,
    pub instalado: bool,
    /// Pasta onde ele está, quando instalado.
    pub pasta: Option<String>,
    pub executavel: Option<String>,
    /// O produto conhece este título de nome.
    pub conhecido: bool,
    /// Matiz de 0 a 359 para a cor do bloco, derivada do id.
    ///
    /// Ver o cabeçalho: o produto não distribui arte de jogo. A cor é estável
    /// e serve para distinguir de relance; quem quiser a capa de verdade
    /// aponta um arquivo da própria máquina.
    pub matiz: u16,
    /// As duas letras do bloco quando não há capa.
    pub iniciais: String,
    /// Por onde pedir a capa de verdade. Ausente em jogo que não é da Steam.
    pub appid: Option<u32>,
}

/// Uma cor estável a partir do identificador.
///
/// Soma simples dos bytes: não precisa ser criptográfica, precisa ser SEMPRE A
/// MESMA. Um jogo que troca de cor a cada abertura faz a grade parecer quebrada
/// mesmo estando certa.
pub fn matiz_de(id: &str) -> u16 {
    let soma: u32 = id.bytes().map(|b| b as u32).sum();
    (soma.wrapping_mul(37) % 360) as u16
}

/// As iniciais do bloco.
///
/// Primeira letra das duas primeiras palavras, ou as duas primeiras letras
/// quando só há uma palavra. Número no começo do nome não vira inicial — "7
/// Days to Die" vira "DD" e não "7D", porque um dígito solto num bloco não
/// identifica nada.
pub fn iniciais_de(nome: &str) -> String {
    let palavras: Vec<&str> = nome
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty() && p.chars().next().is_some_and(|c| c.is_alphabetic()))
        .collect();

    let letras: String = match palavras.as_slice() {
        [] => nome
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(2)
            .collect(),
        [uma] => uma.chars().take(2).collect(),
        [a, b, ..] => [a, b].iter().filter_map(|p| p.chars().next()).collect(),
    };

    letras.to_uppercase()
}

/// Normaliza um caminho para comparação: minúsculas e barras para a frente.
fn achatar(caminho: &str) -> String {
    caminho.to_lowercase().replace('\\', "/")
}

/// Qual jogo do catálogo corresponde a este executável ou pasta.
///
/// O executável manda: ele é o que o sistema usa e o que a otimização por jogo
/// escreve. A pasta é o segundo critério, para as lojas que não informam o
/// executável.
pub fn reconhecer(executavel: Option<&str>, pasta: &str) -> Option<&'static JogoConhecido> {
    let pasta = achatar(pasta);

    if let Some(exe) = executavel {
        let exe = achatar(exe);
        let arquivo = exe.rsplit('/').next().unwrap_or(&exe).to_string();

        if let Some(j) = CATALOGO
            .iter()
            .find(|j| j.executaveis.iter().any(|e| *e == arquivo))
        {
            return Some(j);
        }
    }

    CATALOGO
        .iter()
        .find(|j| j.pastas.iter().any(|p| pasta.contains(p)))
}

/// O que a loja entregou, do jeito que a grade precisa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Detectado {
    pub nome: String,
    pub pasta: String,
    pub executavel: Option<String>,
    /// O número do jogo na Steam, quando vem da Steam.
    pub appid: Option<u32>,
}

/// Monta a grade: o que está instalado, mais o que o produto conhece.
///
/// A ORDEM É A DA TELA, e não a alfabética: instalado primeiro, porque é o que
/// a pessoa veio mexer. Dentro de cada grupo, a ordem do catálogo — que é
/// estável entre aberturas, e uma grade que embaralha a cada abertura faz
/// perder o jogo que já se sabia onde estava.
pub fn montar(detectados: &[Detectado]) -> Vec<NaGrade> {
    let mut grade: Vec<NaGrade> = Vec::new();
    let mut conhecidos_vistos: Vec<&str> = Vec::new();

    for d in detectados {
        let conhecido = reconhecer(d.executavel.as_deref(), &d.pasta);

        // O nome do catálogo ganha do nome da loja: "EA SPORTS FC™ 26" com
        // marca registrada e espaço fino não é um nome, é um logotipo escrito
        // em texto.
        let (id, nome) = match conhecido {
            Some(j) => {
                conhecidos_vistos.push(j.id);
                (j.id.to_string(), j.nome.to_string())
            }
            None => (achatar(&d.pasta), d.nome.clone()),
        };

        // Duas entradas do mesmo jogo acontecem: Steam e a lista do Windows
        // podem apontar o mesmo título. Quem já entrou fica.
        if grade.iter().any(|g| g.id == id) {
            continue;
        }

        grade.push(NaGrade {
            matiz: matiz_de(&id),
            iniciais: iniciais_de(&nome),
            id,
            nome,
            instalado: true,
            pasta: Some(d.pasta.clone()),
            executavel: d.executavel.clone(),
            conhecido: conhecido.is_some(),
            appid: d.appid,
        });
    }

    for j in CATALOGO {
        if conhecidos_vistos.contains(&j.id) {
            continue;
        }

        grade.push(NaGrade {
            id: j.id.to_string(),
            nome: j.nome.to_string(),
            instalado: false,
            pasta: None,
            executavel: None,
            conhecido: true,
            matiz: matiz_de(j.id),
            iniciais: iniciais_de(j.nome),
            // O número do catálogo: é por ele que a capa é procurada, mesmo
            // sem o jogo instalado.
            appid: j.appid,
        });
    }

    grade
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detectado(nome: &str, pasta: &str, exe: Option<&str>) -> Detectado {
        Detectado {
            nome: nome.to_string(),
            pasta: pasta.to_string(),
            executavel: exe.map(|e| e.to_string()),
            appid: None,
        }
    }

    /// O CAMINHO INTEIRO, nesta máquina: varrer → casar → achar a capa.
    ///
    /// Os outros testes provam pedaços. Este prova a junta, que é onde o
    /// defeito mora: um `appid` que se perde entre o manifesto da Steam e a
    /// grade não quebra teste nenhum — só devolve uma tela de letras.
    #[cfg(target_os = "windows")]
    #[test]
    fn a_grade_desta_maquina_chega_com_capa() {
        use crate::modules::capas;
        use crate::modules::windows::jogos;

        let biblioteca = jogos::varrer();
        let Some(raiz) = biblioteca.raiz_steam.clone() else {
            println!("sem Steam nesta máquina");
            return;
        };

        let detectados: Vec<Detectado> = biblioteca
            .jogos
            .iter()
            .map(|j| Detectado {
                nome: j.nome.clone(),
                pasta: j.pasta.to_string_lossy().to_string(),
                executavel: j
                    .executavel
                    .as_ref()
                    .map(|e| e.to_string_lossy().to_string()),
                appid: j.appid,
            })
            .collect();

        let grade = montar(&detectados);
        let instalados: Vec<&NaGrade> = grade.iter().filter(|g| g.instalado).collect();

        println!("instalados: {}", instalados.len());

        let mut com_capa = 0;
        for g in &instalados {
            let capa = g.appid.and_then(|id| capas::procurar_steam(&raiz, id));
            println!(
                "  {} · appid {:?} · capa {}",
                g.nome,
                g.appid,
                capa.as_ref()
                    .map(|c| c
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string())
                    .unwrap_or_else(|| "NENHUMA".to_string())
            );
            if capa.is_some() {
                com_capa += 1;
            }
        }

        println!("com capa: {com_capa} de {}", instalados.len());

        // Sem jogo instalado não há o que afirmar. COM jogo da Steam
        // instalado, o appid TEM de chegar até aqui — se ele se perder no
        // caminho, a grade vira um campo de letras e nenhum outro teste nota.
        let da_steam = instalados.iter().filter(|g| g.appid.is_some()).count();
        if !instalados.is_empty() {
            assert!(
                da_steam > 0 || biblioteca.jogos.iter().all(|j| j.appid.is_none()),
                "há jogo instalado e nenhum appid chegou na grade"
            );
        }
    }

    #[test]
    fn o_catalogo_nao_tem_id_repetido() {
        let mut ids: Vec<&str> = CATALOGO.iter().map(|j| j.id).collect();
        let antes = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), antes, "id repetido no catálogo");
    }

    /// Executável repetido entre dois jogos faria o casamento depender da
    /// ordem da lista — e o jogo errado ganharia a otimização.
    #[test]
    fn nenhum_executavel_pertence_a_dois_jogos() {
        let mut exes: Vec<&str> = CATALOGO
            .iter()
            .flat_map(|j| j.executaveis.iter().copied())
            .collect();
        let antes = exes.len();
        exes.sort_unstable();
        exes.dedup();

        assert_eq!(exes.len(), antes, "executável em dois jogos do catálogo");
    }

    /// Tudo em minúsculas: o casamento compara em minúsculas, e uma entrada
    /// com maiúscula nunca casaria com nada.
    #[test]
    fn o_catalogo_esta_todo_em_minusculas() {
        for j in CATALOGO {
            for e in j.executaveis {
                assert_eq!(
                    *e,
                    e.to_lowercase(),
                    "{} tem executável com maiúscula",
                    j.id
                );
            }
            for p in j.pastas {
                assert_eq!(*p, p.to_lowercase(), "{} tem pasta com maiúscula", j.id);
            }
        }
    }

    #[test]
    fn reconhece_pelo_executavel() {
        let j = reconhecer(
            Some(r"D:\Steam\common\Rust\RustClient.exe"),
            r"D:\Steam\common\Rust",
        )
        .expect("devia reconhecer");

        assert_eq!(j.id, "rust");
    }

    /// A Steam não informa o executável. A pasta é o que sobra.
    #[test]
    fn reconhece_pela_pasta_quando_nao_ha_executavel() {
        let j = reconhecer(
            None,
            r"D:\SteamLibrary\steamapps\common\Counter-Strike Global Offensive",
        )
        .expect("devia reconhecer");

        assert_eq!(j.id, "cs2");
    }

    #[test]
    fn jogo_desconhecido_nao_e_forcado_no_catalogo() {
        assert!(reconhecer(Some("meujogo.exe"), r"C:\Jogos\MeuJogo").is_none());
    }

    /// A regra que a biblioteca existe para sustentar: jogo instalado que o
    /// produto não conhece aparece do mesmo jeito. Prioridade, afinidade e
    /// preferência de placa não dependem de a gente ter ouvido falar dele.
    #[test]
    fn jogo_desconhecido_entra_na_grade_como_instalado() {
        let grade = montar(&[detectado(
            "Meu Jogo Indie",
            r"C:\Jogos\MeuJogo",
            Some("meujogo.exe"),
        )]);

        let meu = grade
            .iter()
            .find(|g| g.nome == "Meu Jogo Indie")
            .expect("na grade");
        assert!(meu.instalado);
        assert!(!meu.conhecido);
    }

    #[test]
    fn instalados_vem_antes_dos_nao_instalados() {
        let grade = montar(&[detectado(
            "Rust",
            r"D:\Steam\common\Rust",
            Some("RustClient.exe"),
        )]);

        assert!(grade[0].instalado, "o instalado tem de abrir a grade");
        assert!(grade.iter().skip(1).all(|g| !g.instalado));
    }

    /// O nome do catálogo ganha do nome da loja.
    #[test]
    fn o_nome_de_tela_vem_do_catalogo() {
        let grade = montar(&[detectado(
            "EA SPORTS FC™ 26",
            r"D:\Games\EA SPORTS FC 26",
            Some("FC26.exe"),
        )]);

        assert_eq!(grade[0].nome, "EA SPORTS FC");
        assert_eq!(grade[0].id, "eafc");
    }

    /// Steam e a lista do Windows podem apontar o mesmo jogo.
    #[test]
    fn o_mesmo_jogo_nao_entra_duas_vezes() {
        let grade = montar(&[
            detectado("Rust", r"D:\Steam\common\Rust", Some("RustClient.exe")),
            detectado("Rust", r"D:\Steam\common\Rust", None),
        ]);

        assert_eq!(grade.iter().filter(|g| g.id == "rust").count(), 1);
    }

    #[test]
    fn todo_jogo_conhecido_aparece_mesmo_sem_nada_instalado() {
        let grade = montar(&[]);

        assert_eq!(grade.len(), CATALOGO.len());
        assert!(grade.iter().all(|g| !g.instalado));
    }

    /// A cor é estável: a mesma entre aberturas, sempre.
    #[test]
    fn a_cor_do_bloco_nao_muda() {
        assert_eq!(matiz_de("cs2"), matiz_de("cs2"));
        assert!(matiz_de("cs2") < 360);
    }

    #[test]
    fn as_iniciais_saem_das_duas_primeiras_palavras() {
        assert_eq!(iniciais_de("Counter-Strike 2"), "CS");
        assert_eq!(iniciais_de("Valorant"), "VA");
        assert_eq!(iniciais_de("Grand Theft Auto V"), "GT");
    }

    /// Dígito no começo não vira inicial: um número solto num bloco não
    /// identifica jogo nenhum.
    #[test]
    fn digito_no_comeco_nao_vira_inicial() {
        assert_eq!(iniciais_de("7 Days to Die"), "DT");
    }

    #[test]
    fn nome_vazio_nao_quebra() {
        assert_eq!(iniciais_de(""), "");
        assert_eq!(iniciais_de("   "), "");
    }
}
