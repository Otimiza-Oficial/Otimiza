// A CONFIGURAÇÃO DO JOGO, POR MOTOR (3.5). O `configjogo.rs` provou o padrão no GTA V/FiveM: a configuração pesa dezenas
// de por cento, os ajustes de Windows alguns. Aqui o mesmo, para outros motores: um FORMATO por motor (o INI da Unreal,
// o KeyValues da Valve) e um PERFIL por jogo em cima dele.
//
// As travas são as do `configjogo.rs`:
// - só troca o VALOR de uma chave que já existe no arquivo do jogo; nunca acrescenta linha. Chave com nome errado no
//   perfil não escreve nada (o arquivo do próprio jogo diz o que existe);
// - recusa com o jogo aberto (ele reescreve o arquivo ao fechar);
// - guarda o arquivo INTEIRO antes (`ChangeRecord::GameConfig`): o desfazer devolve byte a byte.
// Nunca memória do jogo nem executável: só o arquivo de configuração que o próprio jogo grava. Cada perfil cita a ficha
// em `docs/jogos/`, com o que mexe, onde e o que falta confirmar.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formato {
    /// `GameUserSettings.ini` da Unreal: `chave=valor` dentro de `[secao]`.
    IniDaUnreal { secao: &'static str },
    /// `video.txt` da Valve e o `videoconfig.txt` da Respawn: `"chave"  "valor"`.
    KeyValues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mudanca {
    pub chave: &'static str,
    pub valor: &'static str,
    /// Em palavras, o que custa e o que se perde.
    pub por_que: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct PerfilDeJogo {
    /// O id do catálogo (`plataforma::jogos`).
    pub id: &'static str,
    pub nome: &'static str,
    pub formato: Formato,
    pub mudancas: &'static [Mudanca],
    /// O documento com o que o perfil mexe e a fonte.
    pub ficha: &'static str,
}

// ---------------------------------------------------------------------------------------------------------------- INI

/// Pura: o valor de `chave` dentro de `[secao]` (a primeira ocorrência).
pub fn ini_valor<'a>(conteudo: &'a str, secao: &str, chave: &str) -> Option<&'a str> {
    let (inicio, fim) = ini_valor_em(conteudo, secao, chave)?;
    Some(&conteudo[inicio..fim])
}

/// Onde está o valor (início e fim, em bytes) da chave dentro da seção. O fim para antes do `\r` ou `\n`.
fn ini_valor_em(conteudo: &str, secao: &str, chave: &str) -> Option<(usize, usize)> {
    let cabecalho = format!("[{}]", secao);
    let mut dentro = false;
    let mut pos = 0;
    for linha in conteudo.split_inclusive('\n') {
        let limpa = linha.trim_end_matches(['\r', '\n']);
        let aparada = limpa.trim();
        if aparada.starts_with('[') {
            dentro = aparada == cabecalho;
        } else if dentro {
            if let Some((k, _)) = limpa.split_once('=') {
                if k.trim() == chave {
                    let inicio = pos + k.len() + 1;
                    return Some((inicio, pos + limpa.len()));
                }
            }
        }
        pos += linha.len();
    }
    None
}

/// Pura: troca só o valor; o resto do arquivo fica byte a byte. `None` sem a chave.
pub fn ini_trocar(conteudo: &str, secao: &str, chave: &str, novo: &str) -> Option<String> {
    let (inicio, fim) = ini_valor_em(conteudo, secao, chave)?;
    Some(format!("{}{}{}", &conteudo[..inicio], novo, &conteudo[fim..]))
}

// ----------------------------------------------------------------------------------------------------------- KeyValues

/// Onde está o valor (entre as aspas) de `"chave"`, na mesma linha.
fn kv_valor_em(conteudo: &str, chave: &str) -> Option<(usize, usize)> {
    let procurada = format!("\"{}\"", chave);
    let mut desde = 0;
    while let Some(achou) = conteudo[desde..].find(&procurada) {
        let depois = desde + achou + procurada.len();
        let fim_da_linha = conteudo[depois..].find('\n').map_or(conteudo.len(), |i| depois + i);
        let resto = &conteudo[depois..fim_da_linha];
        if let Some(abre) = resto.find('"') {
            if let Some(fecha) = resto[abre + 1..].find('"') {
                let inicio = depois + abre + 1;
                return Some((inicio, inicio + fecha));
            }
        }
        desde = depois;
    }
    None
}

/// Pura: o valor de `"chave"`.
pub fn kv_valor<'a>(conteudo: &'a str, chave: &str) -> Option<&'a str> {
    let (inicio, fim) = kv_valor_em(conteudo, chave)?;
    Some(&conteudo[inicio..fim])
}

/// Pura: troca só o que está entre as aspas do valor. `None` sem a chave.
pub fn kv_trocar(conteudo: &str, chave: &str, novo: &str) -> Option<String> {
    let (inicio, fim) = kv_valor_em(conteudo, chave)?;
    Some(format!("{}{}{}", &conteudo[..inicio], novo, &conteudo[fim..]))
}

// -------------------------------------------------------------------------------------------------------------- perfis

pub const PERFIS: &[PerfilDeJogo] = &[
    PerfilDeJogo {
        id: "fortnite",
        nome: "Fortnite",
        formato: Formato::IniDaUnreal {
            secao: "ScalabilityGroups",
        },
        mudancas: &[
            Mudanca {
                chave: "sg.ShadowQuality",
                valor: "0",
                por_que: "Sombras: das opções mais caras na placa. Desligadas, o jogo perde profundidade, não visão.",
            },
            Mudanca {
                chave: "sg.PostProcessQuality",
                valor: "0",
                por_que: "Pós-processamento (desfoque, brilho): custa e atrapalha a leitura da tela.",
            },
            Mudanca {
                chave: "sg.EffectsQuality",
                valor: "0",
                por_que: "Efeitos (explosões, fumaça): pesam justamente na hora da luta.",
            },
            Mudanca {
                chave: "sg.FoliageQuality",
                valor: "0",
                por_que: "Vegetação: menos grama desenhada, menos custo, e nada a mais escondendo inimigo.",
            },
        ],
        ficha: "docs/jogos/fortnite.md",
    },
    PerfilDeJogo {
        id: "cs2",
        nome: "Counter-Strike 2",
        formato: Formato::KeyValues,
        mudancas: &[
            Mudanca {
                chave: "setting.videocfg_shadow_quality",
                valor: "0",
                por_que: "Qualidade das sombras no mínimo. As sombras dinâmicas dos jogadores ficam como estão: elas \
                          entregam posição.",
            },
            Mudanca {
                chave: "setting.videocfg_ao_detail",
                valor: "0",
                por_que: "Oclusão de ambiente: escurece cantos, custa caro e não ajuda a ver ninguém.",
            },
            Mudanca {
                chave: "setting.videocfg_particle_detail",
                valor: "0",
                por_que: "Partículas: pesam em granada e fumaça, o pior momento para cair FPS.",
            },
        ],
        ficha: "docs/jogos/cs2.md",
    },
    PerfilDeJogo {
        id: "apex",
        nome: "Apex Legends",
        formato: Formato::KeyValues,
        mudancas: &[
            Mudanca {
                chave: "setting.ssao_enabled",
                valor: "0",
                por_que: "Oclusão de ambiente: custa caro e quase não se vê em movimento.",
            },
            Mudanca {
                chave: "setting.volumetric_lighting",
                valor: "0",
                por_que: "Luz volumétrica: bonita e cara, e às vezes esconde inimigo na contraluz.",
            },
        ],
        ficha: "docs/jogos/apex.md",
    },
];

pub fn perfil(id: &str) -> Option<&'static PerfilDeJogo> {
    PERFIS.iter().find(|p| p.id == id)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Previsao {
    pub chave: String,
    pub antes: String,
    pub depois: String,
    pub por_que: String,
}

/// Pura: o que o perfil mudaria NESTE arquivo. Só chave que existe e com valor diferente.
pub fn prever(p: &PerfilDeJogo, conteudo: &str) -> Vec<Previsao> {
    p.mudancas
        .iter()
        .filter_map(|m| {
            let antes = match p.formato {
                Formato::IniDaUnreal { secao } => ini_valor(conteudo, secao, m.chave),
                Formato::KeyValues => kv_valor(conteudo, m.chave),
            }?;
            (antes.trim() != m.valor).then(|| Previsao {
                chave: m.chave.to_string(),
                antes: antes.trim().to_string(),
                depois: m.valor.to_string(),
                por_que: m.por_que.to_string(),
            })
        })
        .collect()
}

/// Pura: o arquivo com o perfil aplicado e as chaves que mudaram.
pub fn aplicar_no_texto(p: &PerfilDeJogo, conteudo: &str) -> (String, Vec<String>) {
    let mut texto = conteudo.to_string();
    let mut mudou = Vec::new();
    for previsto in prever(p, conteudo) {
        let novo = match p.formato {
            Formato::IniDaUnreal { secao } => ini_trocar(&texto, secao, &previsto.chave, &previsto.depois),
            Formato::KeyValues => kv_trocar(&texto, &previsto.chave, &previsto.depois),
        };
        if let Some(n) = novo {
            texto = n;
            mudou.push(previsto.chave);
        }
    }
    (texto, mudou)
}

// ------------------------------------------------------------------------------------------------------- no computador

/// Os arquivos de configuração que o jogo grava, do mais recente para o mais antigo (CS2 tem um por conta Steam).
pub fn arquivos(id: &str) -> Vec<PathBuf> {
    let env = |v: &str| std::env::var(v).ok().map(PathBuf::from);
    let mut achados: Vec<PathBuf> = match id {
        "fortnite" => env("LOCALAPPDATA")
            .map(|l| l.join(r"FortniteGame\Saved\Config\WindowsClient\GameUserSettings.ini"))
            .into_iter()
            .collect(),
        "apex" => env("USERPROFILE")
            .map(|u| u.join(r"Saved Games\Respawn\Apex\local\videoconfig.txt"))
            .into_iter()
            .collect(),
        "cs2" => super::jogos::pasta_da_steam()
            .and_then(|s| std::fs::read_dir(s.join("userdata")).ok())
            .map(|contas| {
                contas
                    .filter_map(|c| c.ok())
                    .map(|c| c.path().join(r"730\local\cfg\cs2_video.txt"))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    achados.retain(|c| c.is_file());
    achados.sort_by_key(|c| {
        std::cmp::Reverse(std::fs::metadata(c).and_then(|m| m.modified()).ok())
    });
    achados
}

/// O jogo está aberto agora (pelos executáveis do catálogo).
pub fn aberto(id: &str) -> bool {
    crate::plataforma::jogos::CATALOGO
        .iter()
        .find(|j| j.id == id)
        .is_some_and(|j| {
            j.executaveis.iter().any(|e| {
                super::frames::encontrar_processo(e.trim_end_matches(".exe")).is_some()
            })
        })
}

#[derive(Debug, Clone, Serialize)]
pub struct JogoNaTela {
    pub id: &'static str,
    pub nome: &'static str,
    pub arquivo: Option<String>,
    pub aberto: bool,
    pub previsao: Vec<Previsao>,
    /// O arquivo existe mas não deu para ler: a tela diz isso em vez de "nada a mudar".
    pub ilegivel: Option<String>,
    pub ficha: &'static str,
}

/// Só lê: cada jogo do catálogo de perfis com o arquivo achado e o que mudaria nele.
pub fn painel() -> Vec<JogoNaTela> {
    PERFIS
        .iter()
        .map(|p| {
            let arquivo = arquivos(p.id).into_iter().next();
            let lido = arquivo.as_ref().map(std::fs::read_to_string);
            let (previsao, ilegivel) = match lido {
                Some(Ok(c)) => (prever(p, &c), None),
                Some(Err(e)) => (Vec::new(), Some(format!("Não consegui ler o arquivo de configuração: {}", e))),
                None => (Vec::new(), None),
            };
            JogoNaTela {
                id: p.id,
                nome: p.nome,
                arquivo: arquivo.map(|a| a.to_string_lossy().to_string()),
                aberto: aberto(p.id),
                previsao,
                ilegivel,
                ficha: p.ficha,
            }
        })
        .collect()
}

pub struct Aplicacao {
    pub jogo: &'static str,
    pub arquivo: PathBuf,
    pub mudou: Vec<String>,
    /// O arquivo inteiro de antes, para o desfazer.
    pub anterior: String,
}

/// Escreve o perfil no arquivo do jogo. Recusa com o jogo aberto; não escreve quando não há o que mudar.
pub fn aplicar(id: &str) -> Result<Aplicacao, String> {
    let p = perfil(id).ok_or_else(|| format!("não conheço a configuração de \"{}\".", id))?;
    if aberto(id) {
        return Err(format!(
            "O {} está aberto. Feche-o antes: ele guarda a configuração em memória e reescreve o arquivo ao sair, \
             apagando o que for mudado agora.",
            p.nome
        ));
    }
    let arquivo = arquivos(id).into_iter().next().ok_or_else(|| {
        format!(
            "Não achei a configuração do {}. Abra o jogo uma vez e feche: ele cria o arquivo.",
            p.nome
        )
    })?;
    let anterior = std::fs::read_to_string(&arquivo)
        .map_err(|e| format!("não consegui ler {}: {}", arquivo.display(), e))?;
    let (novo, mudou) = aplicar_no_texto(p, &anterior);
    if !mudou.is_empty() {
        std::fs::write(&arquivo, &novo)
            .map_err(|e| format!("não consegui escrever em {}: {}", arquivo.display(), e))?;
    }
    Ok(Aplicacao {
        jogo: p.nome,
        arquivo,
        mudou,
        anterior,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const INI: &str = "[/Script/FortniteGame.FortGameUserSettings]\r\nFrameRateLimit=0.000000\r\n\r\n[ScalabilityGroups]\r\nsg.ResolutionQuality=100.000000\r\nsg.ShadowQuality=3\r\nsg.PostProcessQuality=3\r\nsg.EffectsQuality=0\r\nsg.FoliageQuality=3\r\n\r\n[Outra]\r\nsg.ShadowQuality=9\r\n";

    const KV: &str = "\"video.cfg\"\n{\n\t\"Version\"\t\t\"16\"\n\t\"setting.videocfg_shadow_quality\"\t\t\"2\"\n\t\"setting.videocfg_ao_detail\"\t\t\"1\"\n\t\"setting.videocfg_particle_detail\"\t\t\"0\"\n}\n";

    #[test]
    fn ini_le_so_dentro_da_secao() {
        assert_eq!(ini_valor(INI, "ScalabilityGroups", "sg.ShadowQuality"), Some("3"));
        assert_eq!(ini_valor(INI, "Outra", "sg.ShadowQuality"), Some("9"));
        assert_eq!(ini_valor(INI, "ScalabilityGroups", "sg.NaoExiste"), None);
    }

    #[test]
    fn ini_troca_so_o_valor_e_preserva_o_fim_de_linha() {
        let n = ini_trocar(INI, "ScalabilityGroups", "sg.ShadowQuality", "0").unwrap();
        assert!(n.contains("sg.ShadowQuality=0\r\n"));
        assert!(n.contains("[Outra]\r\nsg.ShadowQuality=9\r\n"), "a outra seção fica");
        assert_eq!(n.len(), INI.len(), "\"3\" vira \"0\": o resto do arquivo fica igual");
    }

    #[test]
    fn kv_le_e_troca_entre_as_aspas() {
        assert_eq!(kv_valor(KV, "setting.videocfg_shadow_quality"), Some("2"));
        let n = kv_trocar(KV, "setting.videocfg_ao_detail", "0").unwrap();
        assert!(n.contains("\t\"setting.videocfg_ao_detail\"\t\t\"0\"\n"));
        assert_eq!(n.len(), KV.len());
        assert_eq!(kv_trocar(KV, "setting.inexistente", "0"), None);
    }

    #[test]
    fn kv_nao_confunde_chave_que_comeca_igual() {
        let t = "\t\"setting.a_b\"\t\"1\"\n\t\"setting.a\"\t\"7\"\n";
        assert_eq!(kv_valor(t, "setting.a"), Some("7"));
    }

    #[test]
    fn so_muda_o_que_existe_e_esta_diferente() {
        let fortnite = perfil("fortnite").unwrap();
        let p = prever(fortnite, INI);
        let chaves: Vec<&str> = p.iter().map(|x| x.chave.as_str()).collect();
        // EffectsQuality já está em 0: não muda. As outras três existem e estão em 3.
        assert_eq!(chaves, ["sg.ShadowQuality", "sg.PostProcessQuality", "sg.FoliageQuality"]);
        let (novo, mudou) = aplicar_no_texto(fortnite, INI);
        assert_eq!(mudou.len(), 3);
        assert_eq!(ini_valor(&novo, "ScalabilityGroups", "sg.ShadowQuality"), Some("0"));
        assert_eq!(ini_valor(&novo, "Outra", "sg.ShadowQuality"), Some("9"));
    }

    #[test]
    fn arquivo_sem_as_chaves_nao_ganha_linha_nova() {
        let apex = perfil("apex").unwrap();
        let t = "\"VideoConfig\"\n{\n\t\"setting.outra\"\t\t\"1\"\n}\n";
        let (novo, mudou) = aplicar_no_texto(apex, t);
        assert!(mudou.is_empty());
        assert_eq!(novo, t, "nenhum byte muda");
    }

    #[test]
    fn aplicar_duas_vezes_nao_muda_mais_nada() {
        let cs2 = perfil("cs2").unwrap();
        let (uma, _) = aplicar_no_texto(cs2, KV);
        let (duas, mudou) = aplicar_no_texto(cs2, &uma);
        assert_eq!(uma, duas);
        assert!(mudou.is_empty());
    }

    #[test]
    fn todo_perfil_esta_no_catalogo_e_tem_ficha() {
        for p in PERFIS {
            assert!(
                crate::plataforma::jogos::CATALOGO.iter().any(|j| j.id == p.id),
                "{} não está no catálogo de jogos",
                p.id
            );
            assert!(p.ficha.starts_with("docs/jogos/"));
            let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            assert!(raiz.join(p.ficha).is_file(), "falta a ficha {}", p.ficha);
            assert!(!p.mudancas.is_empty());
            for m in p.mudancas {
                assert!(!m.por_que.trim().is_empty(), "{}: {} sem o porquê", p.id, m.chave);
            }
        }
    }

    #[test]
    fn o_perfil_nao_toca_o_que_entrega_posicao_no_cs2() {
        let cs2 = perfil("cs2").unwrap();
        assert!(
            !cs2.mudancas.iter().any(|m| m.chave.contains("dynamic_shadows")),
            "as sombras dinâmicas mostram o inimigo: ficam como a pessoa deixou"
        );
    }
}
