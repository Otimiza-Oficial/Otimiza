// O CitizenFX.ini — o motor do cliente FiveM, nunca escrito
//
// O produto já lê o `gta5_settings.xml` (`configjogo.rs`), que é a
// configuração GRÁFICA do jogo. Este módulo lê um arquivo diferente: o
// `CitizenFX.ini`, que é a configuração do MOTOR DO CLIENTE FiveM — não do
// GTA V em si.
//
// A CORREÇÃO QUE MOTIVOU ESTE MÓDULO
//
// Uma pesquisa anterior, feita a partir de blog de hospedagem, errou três dos
// quatro detalhes: o caminho, o nome da chave e o formato do valor. Verificado
// de novo na máquina do dono, contra a fonte oficial (Cfx.re):
//
//   caminho  `%LOCALAPPDATA%\FiveM\FiveM.app\CitizenFX.ini` (não `%APPDATA%`)
//   chave    `PoolSizesIncrease` (não `PoolSize`)
//   valor    um JSON de pool para acréscimo (não um número solto)
//
// O arquivo do dono tem a chave, na seção `[Game]`, e ela está VAZIA.
//
// POR QUE SÓ LÊ, E POR QUE ISTO NÃO É PROVISÓRIO
//
// Aumentar um pool custa memória, e o sintoma de pool estourado tem texto
// próprio no registro do FiveM — mas não há nenhum estouro nos registros da
// máquina que motivou este módulo. Escrever um detector para uma frase nunca
// observada é adivinhar: ou nunca detecta, ou detecta errado e mexe no
// arquivo do jogo do cliente sem motivo. A oferta de aumento só entra quando
// um cliente com o problema mandar o registro real.
//
// Até lá, o valor inteiro deste módulo é mostrar o que já está configurado —
// que costuma ser nada, e "nada configurado" é um resultado bom e comum, do
// mesmo jeito que "nenhuma corrupção encontrada" é um bom resultado do
// reparo. Ele não é motivo para sugerir que o cliente aumente nada: o produto
// ainda não sabe se aumentar ajudaria NESTA máquina.
//
// POR QUE É UM MÓDULO SEPARADO, E NÃO MAIS CÓDIGO DENTRO DE `fivem.rs`
//
// `fivem.rs` já sabe onde o FiveM instala (`pasta_do_fivem`) e este módulo
// reaproveita isso — o `.ini` mora dentro da mesma pasta que `fivem.rs` já
// resolve. Mas o assunto aqui é outro: um formato de arquivo diferente (INI
// com um campo que carrega JSON dentro) e uma lista de tetos publicada pela
// Cfx.re. É a mesma divisão que já existe entre `fivem.rs` (pastas e
// processos) e `configjogo.rs` (o arquivo de configuração do jogo) — dois
// módulos, um dono cada um, mesmo os dois falando do mesmo jogo.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Um pool com acréscimo configurado, e o teto que a lista oficial permite —
/// quando o pool está na lista que este módulo conhece.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolAjustado {
    pub nome: String,
    pub aumento: u64,
    /// `None` quando o pool não está no recorte estático abaixo — não
    /// significa que o pool não exista, só que não foi conferido aqui.
    pub teto_conhecido: Option<u64>,
}

/// O estado de `PoolSizesIncrease`, já interpretado.
///
/// `#[serde(tag = "status")]` marca o campo internamente — é o que deixa a
/// tela decidir por CAMPO ESTRUTURADO (`pool_sizes.status`) e nunca por
/// comparar a prosa da `note`. A guarda
/// `a_tela_nao_decide_cor_comparando_texto_do_backend`, em `commands.rs`,
/// reprova o build se `main.ts` voltar a decidir assim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum PoolSizesIncrease {
    /// A chave existe e está vazia (ou não existe na seção) — o caso comum,
    /// e não é um problema.
    Vazio,
    /// A chave tem pools configurados, com o que cada um pede.
    Configurado { pools: Vec<PoolAjustado> },
    /// A chave tem um valor, mas não é o JSON esperado. Dito explicitamente:
    /// tratar isto como vazio seria esconder um arquivo que outra ferramenta
    /// (ou uma edição manual) deixou num formato que este produto não
    /// entende.
    Invalido { bruto: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitizenFxReport {
    pub existe: bool,
    pub caminho: Option<PathBuf>,
    /// `None` quando o arquivo não existe ou não pôde ser lido — "não sei".
    /// Nunca vira `Some(Vazio)` nesse caso: "não consegui ler" e "sei que
    /// está vazio" são coisas diferentes, e confundi-las já foi o defeito
    /// desta versão duas vezes em outros módulos.
    pub pool_sizes: Option<PoolSizesIncrease>,
    pub note: String,
}

/// Recorte ESTÁTICO da lista oficial de tetos por pool.
///
/// Fonte: <https://content.cfx.re/mirrors/client/pool-size-limits/fivem.json>,
/// conferida em 2026-09-03. Não é buscada em tempo de execução — isso
/// adicionaria uma dependência de rede a uma tela que só lê um arquivo local,
/// e o produto já pesou com cuidado a única chamada de rede que tem
/// (`atualizacao.rs`). Um teto desatualizado aqui não decide nada: é só
/// referência ao lado do que o cliente já configurou. Só os pools citados na
/// correção do desenho entraram — não é a lista inteira.
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

/// Lê o valor de `chave=` dentro da seção `[secao]` de um `.ini`.
///
/// Analisador de uma passada, sem biblioteca de INI: o arquivo mistura seções
/// (`[Game]`, `[Addons]`) e como este módulo só LÊ, trazer uma dependência
/// nova para o instalador só para nunca escrever de volta não se paga. Devolve
/// `None` quando a seção ou a chave não existem — e `None` aqui vira
/// `PoolSizesIncrease::Vazio` no chamador, porque "chave ausente" e "chave
/// vazia" pedem o mesmo tratamento: não há nada configurado.
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

/// Interpreta o conteúdo JÁ LIDO do `.ini`. **Função pura** — é o que permite
/// provar os três estados (vazio, configurado, inválido) com texto sintético,
/// sem nada em disco.
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
        // Presente mas ilegível: dito explicitamente, nunca escondido atrás
        // de "vazio". Ver o cabeçalho do módulo — este é o defeito que este
        // desenho existe para não repetir.
        Err(_) => PoolSizesIncrease::Invalido {
            bruto: bruto.to_string(),
        },
    }
}

/// Decide o estado do pool a partir do RESULTADO de ler o arquivo, e não do
/// conteúdo direto.
///
/// **Função pura**, e é o ponto exato onde o defeito de colapsar "não
/// consegui ler" em "sei que está vazio" poderia entrar. `Err` devolve
/// `None` — "não sei" — e nunca `Some(PoolSizesIncrease::Vazio)`.
fn estado_do_pool(leitura: Result<&str, ()>) -> Option<PoolSizesIncrease> {
    let conteudo = leitura.ok()?;
    Some(pool_sizes_de(conteudo))
}

fn caminho_do_ini() -> Option<PathBuf> {
    super::fivem::pasta_do_fivem().map(|base| base.join("CitizenFX.ini"))
}

/// Por que a leitura falhou, em português, a partir do erro REAL do sistema.
///
/// A frase anterior era uma só, para qualquer falha: "pode ser permissão de
/// arquivo — feche o FiveM e tente de novo". Ela ATRIBUÍA uma causa que nunca
/// foi verificada, e o conselho que dava não funcionaria nunca no caso mais
/// provável aqui: o `CitizenFX.ini` é escrito pelo cliente do FiveM, e basta
/// um `IVPath=C:\Jogos\GTA V — Cópia` gravado em ANSI, ou o arquivo salvo em
/// UTF-16 com BOM (o que as APIs de INI do Windows produzem), para
/// `read_to_string` devolver `InvalidData`. Pasta com acento é o caso comum
/// no Brasil, não o exótico — o cliente fecharia o FiveM, tentaria de novo, e
/// leria a mesma frase para sempre.
///
/// O caminho de CBS deste mesmo lançamento já faz assim. Dois arquivos da
/// mesma versão não podem ter suposições contraditórias sobre o que significa
/// "não deu para ler".
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

        // O CASO MAIS COMUM, E É UM BOM RESULTADO — não um problema a
        // resolver, do mesmo jeito que "nenhuma corrupção encontrada" no
        // reparo. NÃO sugere aumentar nada: o produto ainda não sabe se
        // aumentar ajudaria nesta máquina, porque não há evidência de que
        // algum pool estourou.
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

/// Levantamento completo do `CitizenFX.ini`. Só leitura.
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

    /// O ARQUIVO REAL DO DONO: seção `[Game]`, chave presente, vazia.
    const INI_DO_DONO: &str = "[Game]\r\n\
IVPath=\r\n\
DefaultBuild=\r\n\
SavedBuildNumber=3570\r\n\
UpdateChannel=production\r\n\
PoolSizesIncrease=\r\n\
\r\n\
[Addons]\r\n\
";

    /// CANÁRIO: falha de leitura não pode virar "sei que está vazio".
    ///
    /// Este é o defeito que já apareceu duas vezes nesta versão em outros
    /// módulos (`cbslog`, `veredito`): "não consegui saber" sendo relatado
    /// como se fosse "sei, e a resposta é nada". Se `estado_do_pool` for
    /// reescrita para tratar erro como conteúdo vazio (`unwrap_or("")`, por
    /// exemplo), esta primeira asserção passa a falhar.
    #[test]
    fn falha_de_leitura_nunca_vira_estado_vazio() {
        let falhou = estado_do_pool(Err(()));
        assert!(
            falhou.is_none(),
            "uma leitura que falhou virou um estado conhecido: {:?}",
            falhou
        );

        // E o caso que É realmente vazio continua distinto: aqui a leitura
        // TEVE sucesso, e o conteúdo diz que a chave está vazia.
        let vazio_de_verdade = estado_do_pool(Ok(INI_DO_DONO));
        assert_eq!(vazio_de_verdade, Some(PoolSizesIncrease::Vazio));
    }

    #[test]
    fn arquivo_do_dono_hoje_esta_vazio() {
        assert_eq!(pool_sizes_de(INI_DO_DONO), PoolSizesIncrease::Vazio);
    }

    #[test]
    fn chave_ausente_da_secao_tambem_e_vazio() {
        // Sem a linha `PoolSizesIncrease=` de jeito nenhum — versão mais
        // antiga do FiveM, por exemplo. Ausência e vazio pedem o mesmo
        // tratamento: não há nada configurado.
        let ini = "[Game]\r\nDefaultBuild=\r\n";
        assert_eq!(pool_sizes_de(ini), PoolSizesIncrease::Vazio);
    }

    #[test]
    fn valor_fora_do_json_nao_vira_vazio() {
        // O caso que este módulo existe para não esconder: um valor
        // presente, mas que não é o formato esperado.
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
                // Não estar no recorte estático não é erro — só não temos o
                // teto para mostrar ao lado.
                assert_eq!(desconhecido.teto_conhecido, None);
            }
            outro => panic!("pools configurados viraram {:?}", outro),
        }
    }

    #[test]
    fn le_apenas_a_secao_pedida() {
        // Uma chave de mesmo nome fora de `[Game]` não pode ser lida no
        // lugar da de dentro — leitura por seção, não por chave solta.
        let ini = "[Addons]\r\nPoolSizesIncrease={\"CMoveObject\": 999}\r\n\r\n[Game]\r\nPoolSizesIncrease=\r\n";
        assert_eq!(pool_sizes_de(ini), PoolSizesIncrease::Vazio);
    }

    /// O produto nunca sugere aumentar nada: a mensagem do caso comum não
    /// pode conter a palavra que convidaria a isso.
    #[test]
    fn nota_do_vazio_nao_sugere_aumentar_nada() {
        let nota = montar_nota(&Some(PoolSizesIncrease::Vazio), None);

        assert!(!nota.to_lowercase().contains("aumente"));
        assert!(!nota.to_lowercase().contains("recomendo"));
        // E precisa dizer que é normal, na mesma voz do "nenhuma corrupção
        // encontrada" do reparo — um bom resultado comum, não uma pendência.
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

        // O arquivo relatado como existente precisa existir de verdade.
        if r.existe {
            let caminho = r.caminho.as_ref().expect("existe=true sem caminho");
            assert!(caminho.is_file());
            // Leitura bem-sucedida sempre resolve para um estado conhecido.
            assert!(r.pool_sizes.is_some());
        }
    }

    /// Só o código de produção. Os guards abaixo procuram termos proibidos no
    /// módulo, e olhar o arquivo inteiro faria os testes se acusarem
    /// sozinhos.
    fn codigo_de_producao() -> &'static str {
        let fonte = include_str!("citizenfx.rs");
        fonte.split("#[cfg(test)]").next().unwrap()
    }

    /// Este módulo é SÓ LEITURA. Nenhuma escrita no arquivo do cliente.
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
