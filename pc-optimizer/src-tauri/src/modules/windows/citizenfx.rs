// Lê o `CitizenFX.ini`, o motor do cliente FiveM (a parte gráfica é `configjogo.rs`). NUNCA escreve. Conferido na
// fonte oficial (Cfx.re), depois de uma pesquisa de blog errar três de quatro detalhes: caminho
// `%LOCALAPPDATA%\FiveM\FiveM.app\CitizenFX.ini`, chave `PoolSizesIncrease` em `[Game]`, valor JSON. Só lê porque
// não há estouro de pool observado: detector para frase nunca vista é adivinhar. "Nada configurado" é bom e comum.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolAjustado {
    pub nome: String,
    pub aumento: u64,
    /// `None`: fora do recorte estático, não conferido aqui.
    pub teto_conhecido: Option<u64>,
}

/// `tag = "status"` para a tela decidir por campo, nunca pela prosa; a guarda
/// `a_tela_nao_decide_cor_comparando_texto_do_backend` (`commands.rs`) cobra isso.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum PoolSizesIncrease {
    Vazio,
    Configurado { pools: Vec<PoolAjustado> },
    /// Dito explicitamente: tratar como vazio esconderia um arquivo num formato que não se entende.
    Invalido { bruto: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitizenFxReport {
    pub existe: bool,
    pub caminho: Option<PathBuf>,
    /// `None` é "não sei", nunca `Some(Vazio)`.
    pub pool_sizes: Option<PoolSizesIncrease>,
    pub note: String,
}

/// Recorte ESTÁTICO de <https://content.cfx.re/mirrors/client/pool-size-limits/fivem.json> (2026-09-03). Não é
/// buscado em execução: seria rede numa tela que só lê um arquivo local, e o teto aqui é só referência.
const TETOS_CONHECIDOS: &[(&str, u64)] = &[
    ("CMoveObject", 600),
    ("FragmentStore", 30_000),
    ("TxdStore", 50_000),
];

fn teto_conhecido(pool: &str) -> Option<u64> {
    TETOS_CONHECIDOS
        .iter()
        .find(|(nome, _)| *nome == pool)
        .map(|(_, teto)| *teto)
}

/// Sem biblioteca de INI: só lê. `None` vira `Vazio` no chamador (ausente e vazia: nada configurado).
fn ler_chave(conteudo: &str, secao: &str, chave: &str) -> Option<String> {
    let alvo_secao = format!("[{}]", secao);
    let prefixo_chave = format!("{}=", chave);
    let mut dentro = false;

    for linha in conteudo.lines() {
        let linha = linha.trim();

        if linha.starts_with('[') && linha.ends_with(']') {
            dentro = linha == alvo_secao;
            continue;
        }

        if !dentro {
            continue;
        }

        if let Some(resto) = linha.strip_prefix(&prefixo_chave) {
            return Some(resto.trim().to_string());
        }
    }

    None
}

fn pool_sizes_de(conteudo: &str) -> PoolSizesIncrease {
    let bruto = ler_chave(conteudo, "Game", "PoolSizesIncrease").unwrap_or_default();
    let bruto = bruto.trim();

    if bruto.is_empty() {
        return PoolSizesIncrease::Vazio;
    }

    match serde_json::from_str::<BTreeMap<String, u64>>(bruto) {
        Ok(mapa) if mapa.is_empty() => PoolSizesIncrease::Vazio,
        Ok(mapa) => {
            let pools = mapa
                .into_iter()
                .map(|(nome, aumento)| {
                    let teto_conhecido = teto_conhecido(&nome);
                    PoolAjustado {
                        nome,
                        aumento,
                        teto_conhecido,
                    }
                })
                .collect();

            PoolSizesIncrease::Configurado { pools }
        }
        Err(_) => PoolSizesIncrease::Invalido {
            bruto: bruto.to_string(),
        },
    }
}

/// `Err` devolve `None` ("não sei"), nunca `Some(Vazio)`.
fn estado_do_pool(leitura: Result<&str, ()>) -> Option<PoolSizesIncrease> {
    let conteudo = leitura.ok()?;
    Some(pool_sizes_de(conteudo))
}

fn caminho_do_ini() -> Option<PathBuf> {
    super::fivem::pasta_do_fivem().map(|base| base.join("CitizenFX.ini"))
}

/// Pelo erro REAL: a frase única de "permissão" nunca funcionaria no caso mais comum, o arquivo em ANSI (pasta
/// com acento) ou UTF-16 com BOM, que dá `InvalidData` em `read_to_string`.
fn explicar_falha(erro: &std::io::ErrorKind) -> &'static str {
    match erro {
        std::io::ErrorKind::PermissionDenied => {
            "O Windows negou acesso ao arquivo — feche o FiveM e tente de novo."
        }
        std::io::ErrorKind::NotFound => {
            "O arquivo sumiu entre a checagem e a leitura, o que costuma ser o FiveM \
             reescrevendo-o no mesmo instante. Tente de novo."
        }
        std::io::ErrorKind::InvalidData => {
            "O arquivo não está em UTF-8 — costuma ser acento no caminho do jogo gravado na \
             codificação antiga do Windows. Não é problema no seu PC, e não há o que fazer: \
             o Otimiza simplesmente não lê este arquivo."
        }
        _ => "Não consegui abrir o arquivo.",
    }
}

fn montar_nota(pool_sizes: &Option<PoolSizesIncrease>, falha: Option<std::io::ErrorKind>) -> String {
    match pool_sizes {
        None => format!(
            "Não consegui ler o CitizenFX.ini. {} O arquivo não foi tocado.",
            falha
                .as_ref()
                .map(explicar_falha)
                .unwrap_or("Não consegui abrir o arquivo.")
        ),

        // Não sugere aumentar nada: não há evidência de pool estourado.
        Some(PoolSizesIncrease::Vazio) => {
            "Nenhum pool com acréscimo configurado. É o normal — a maioria das instalações de \
             FiveM nunca precisa mexer aqui, e isto não é um problema para resolver."
                .to_string()
        }

        Some(PoolSizesIncrease::Invalido { .. }) => {
            "O CitizenFX.ini tem um valor em PoolSizesIncrease que não é o JSON esperado (um \
             mapa de pool para acréscimo). Não consigo interpretar este valor — pode ter sido \
             escrito por outra ferramenta ou editado à mão. O arquivo não foi tocado."
                .to_string()
        }

        Some(PoolSizesIncrease::Configurado { pools }) => {
            format!(
                "{} pool(s) com acréscimo já configurado nesta máquina. Pode ter sido você ou \
                 outra ferramenta — o Otimiza só está mostrando o que já está lá.",
                pools.len()
            )
        }
    }
}

pub fn analyze() -> CitizenFxReport {
    let Some(caminho) = caminho_do_ini() else {
        return CitizenFxReport {
            existe: false,
            caminho: None,
            pool_sizes: None,
            note: "O FiveM não está instalado nesta máquina — não há CitizenFX.ini para ler."
                .to_string(),
        };
    };

    if !caminho.is_file() {
        return CitizenFxReport {
            existe: false,
            caminho: Some(caminho),
            pool_sizes: None,
            note: "O CitizenFX.ini ainda não existe nesta instalação do FiveM.".to_string(),
        };
    }

    let leitura = std::fs::read_to_string(&caminho);
    let falha = leitura.as_ref().err().map(|e| e.kind());
    let pool_sizes = estado_do_pool(leitura.as_deref().map_err(|_| ()));
    let note = montar_nota(&pool_sizes, falha);

    CitizenFxReport {
        existe: true,
        caminho: Some(caminho),
        pool_sizes,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INI_DO_DONO: &str = "[Game]\r\n\
IVPath=\r\n\
DefaultBuild=\r\n\
SavedBuildNumber=3570\r\n\
UpdateChannel=production\r\n\
PoolSizesIncrease=\r\n\
\r\n\
[Addons]\r\n\
";

    /// CANÁRIO: se `estado_do_pool` voltar a tratar erro como vazio (`unwrap_or("")`), isto falha.
    #[test]
    fn falha_de_leitura_nunca_vira_estado_vazio() {
        let falhou = estado_do_pool(Err(()));
        assert!(
            falhou.is_none(),
            "uma leitura que falhou virou um estado conhecido: {:?}",
            falhou
        );

        let vazio_de_verdade = estado_do_pool(Ok(INI_DO_DONO));
        assert_eq!(vazio_de_verdade, Some(PoolSizesIncrease::Vazio));
    }

    #[test]
    fn arquivo_do_dono_hoje_esta_vazio() {
        assert_eq!(pool_sizes_de(INI_DO_DONO), PoolSizesIncrease::Vazio);
    }

    #[test]
    fn chave_ausente_da_secao_tambem_e_vazio() {
        let ini = "[Game]\r\nDefaultBuild=\r\n";
        assert_eq!(pool_sizes_de(ini), PoolSizesIncrease::Vazio);
    }

    #[test]
    fn valor_fora_do_json_nao_vira_vazio() {
        let ini = "[Game]\r\nPoolSizesIncrease=isto nao e json\r\n";

        match pool_sizes_de(ini) {
            PoolSizesIncrease::Invalido { bruto } => {
                assert_eq!(bruto, "isto nao e json");
            }
            outro => panic!("valor não parseável virou {:?}, não Invalido", outro),
        }
    }

    #[test]
    fn pools_configurados_saem_com_teto_conhecido_quando_existe() {
        let ini = r#"[Game]
PoolSizesIncrease={"CMoveObject": 600, "FragmentStore": 30000, "PoolQueNaoConhecemos": 10}
"#;

        match pool_sizes_de(ini) {
            PoolSizesIncrease::Configurado { pools } => {
                assert_eq!(pools.len(), 3);

                let cmove = pools.iter().find(|p| p.nome == "CMoveObject").unwrap();
                assert_eq!(cmove.aumento, 600);
                assert_eq!(cmove.teto_conhecido, Some(600));

                let desconhecido = pools.iter().find(|p| p.nome == "PoolQueNaoConhecemos").unwrap();
                assert_eq!(desconhecido.aumento, 10);
                assert_eq!(desconhecido.teto_conhecido, None);
            }
            outro => panic!("pools configurados viraram {:?}", outro),
        }
    }

    #[test]
    fn le_apenas_a_secao_pedida() {
        // Leitura por seção, não por chave solta.
        let ini = "[Addons]\r\nPoolSizesIncrease={\"CMoveObject\": 999}\r\n\r\n[Game]\r\nPoolSizesIncrease=\r\n";
        assert_eq!(pool_sizes_de(ini), PoolSizesIncrease::Vazio);
    }

    #[test]
    fn nota_do_vazio_nao_sugere_aumentar_nada() {
        let nota = montar_nota(&Some(PoolSizesIncrease::Vazio), None);

        assert!(!nota.to_lowercase().contains("aumente"));
        assert!(!nota.to_lowercase().contains("recomendo"));
        assert!(nota.contains("normal") || nota.contains("não é um problema"));
    }

    #[test]
    fn nota_do_invalido_diz_que_nao_entendeu_em_vez_de_ficar_calada() {
        let nota = montar_nota(
            &Some(PoolSizesIncrease::Invalido {
                bruto: "x".to_string(),
            }),
            None,
        );

        assert!(nota.contains("não") && (nota.contains("interpretar") || nota.contains("entend")));
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();

        println!("existe: {} | nota: {}", r.existe, r.note);
        if let Some(caminho) = &r.caminho {
            println!("  caminho: {}", caminho.display());
        }

        assert!(!r.note.is_empty());

        if r.existe {
            let caminho = r.caminho.as_ref().expect("existe=true sem caminho");
            assert!(caminho.is_file());
            assert!(r.pool_sizes.is_some());
        }
    }

    /// Só o código de produção: senão os testes se acusariam pelos termos proibidos.
    fn codigo_de_producao() -> &'static str {
        let fonte = include_str!("citizenfx.rs");
        fonte.split("#[cfg(test)]").next().unwrap()
    }

    #[test]
    fn modulo_nunca_escreve_no_arquivo() {
        let producao = codigo_de_producao();

        assert!(
            !producao.contains("fs::write") && !producao.contains("File::create"),
            "este módulo não pode escrever no CitizenFX.ini — a decisão de aumentar um pool \
             sem evidência de estouro é exatamente o \"aplique e torça\" que o produto recusa"
        );
    }
}
