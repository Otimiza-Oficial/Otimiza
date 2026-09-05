// Mapa de pastas: onde o disco foi parar
//
// O liberador de espaço que já existe responde "o que dá para apagar com
// segurança". Falta a pergunta anterior, que é a que o dono do PC realmente
// faz: "meu disco tem 500 GB, eu não baixei nada, cadê o espaço?".
//
// Sem essa resposta o técnico limpa 2 GB de temporários num disco que tem 400 GB
// de jogos esquecidos e backup de celular, e o cliente continua sem espaço.
// Aqui não apagamos nada — só mostramos, do maior para o menor, com o caminho
// exato para a pessoa decidir.
//
// TRÊS ARMADILHAS QUE ESTE MÓDULO PRECISA DESVIAR
//
// 1. Ponto de nova análise (junction, link simbólico). O Windows é cheio deles:
//    `C:\Users\Fulano\Documents\Minha Música` aponta para outro lugar, e
//    `C:\Documents and Settings` aponta para `C:\Users`. Seguir esses links
//    conta o mesmo arquivo várias vezes e, no pior caso, entra em laço infinito.
//
// 2. Pasta sem permissão. Metade de `C:\Windows` e todo perfil de outro usuário
//    devolvem erro. Isso é normal e não é falha da varredura.
//
// 3. Tempo. Varrer um disco cheio percorre milhões de arquivos — sete minutos
//    na máquina onde isto foi escrito. A varredura tem prazo e avisa quando
//    parou nele, em vez de devolver um número menor fingindo que é o total.

use serde::{Deserialize, Serialize};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Atributo que marca junction e link simbólico no Windows.
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

/// Trava contra estrutura patológica ou link que escapou da checagem.
///
/// Não é o limite que importa na prática — quem manda é o tempo. Profundidade
/// baixa demais dá número errado: `node_modules` e caches de compilador passam
/// fácil de vinte níveis, e cortar ali marcaria como "parcial" quase todas as
/// pastas, transformando o aviso em ruído que ninguém lê.
const PROFUNDIDADE_MAXIMA: u32 = 40;

/// Quanto tempo a varredura inteira pode levar.
///
/// Somar um perfil de verdade percorre milhões de arquivos: nesta máquina, sem
/// limite, levou sete minutos. Ninguém espera sete minutos olhando uma tela
/// parada, e um botão que parece travado é pior que um número aproximado.
const ORCAMENTO_SEGUNDOS: u64 = 45;

/// O prazo é dividido entre as pastas, e não gasto por ordem de chegada.
///
/// A primeira versão tinha um prazo único para tudo. O efeito foi pior que a
/// lentidão que ele resolvia: as pastas são lidas em ordem alfabética, o tempo
/// acabava nas primeiras, e o mapa anunciava `.cache` com 1,4 GB como a maior
/// pasta do perfil — enquanto `AppData`, com 151 GB, nem chegava a ser aberta.
/// Um número errado apresentado com confiança é pior que nenhum número.
///
/// Dando a cada pasta uma fatia do prazo, todas são medidas. As que terminam
/// antes devolvem o tempo que sobrou para as seguintes, então uma varredura que
/// caberia no orçamento continua completa.
fn fatia(restante: std::time::Duration, pastas_faltando: usize) -> std::time::Duration {
    restante / pastas_faltando.max(1) as u32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderEntry {
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub formatted: String,
    /// Fração do total varrido, de 0 a 100. É o que transforma a lista num mapa.
    pub percent: f64,
    /// Explicação em português do que costuma morar ali, quando a pasta é
    /// conhecida. Vazio para pasta que o usuário criou.
    pub explanation: String,
    /// Verdadeiro quando a soma parou no limite de profundidade, no prazo, ou
    /// esbarrou em permissão negada em algum descendente — em qualquer um
    /// desses casos o número mostrado é um piso, não o total.
    pub partial: bool,
    /// O que esta linha é: limpável, do cliente, ou ilegível. Ver `Natureza`.
    pub natureza: Natureza,
}

/// O que cada linha do mapa É, para a tela não precisar adivinhar.
///
/// CAMPO TIPADO, E NÃO FRASE. Este projeto reprova o build quando a interface
/// decide comparando prosa vinda do backend — já aconteceu três vezes, e a
/// guarda em `commands.rs` existe por causa disso.
///
/// E são TRÊS estados, não dois. "Não consegui ler" precisa ser distinto de
/// "não há nada aqui": uma pasta sem permissão contada como zero faria o total
/// mentir, e o cliente concluiria que o espaço sumiu no nada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tipo")]
pub enum Natureza {
    /// Categoria que o `diskspace.rs` sabe limpar. Ganha botão na tela.
    PodeLimpar,
    /// Arquivo do cliente. O produto MOSTRA o caminho e não faz mais nada.
    Seu,
    /// Não deu para ler o próprio nível 1 — falha de permissão nesta pasta
    /// mesma, e não em algum descendente (ver `nivel1_foi_lido`) nem no
    /// prazo da varredura (esse é `partial`, campo separado — ver
    /// `FolderEntry::partial`).
    NaoSei,
}

/// Prefixos que o `diskspace.rs` já sabe limpar.
///
/// A lista mora aqui em minúsculas porque caminho no Windows não diferencia
/// maiúscula de minúscula, e comparar sem normalizar deixaria `C:\WINDOWS\TEMP`
/// passar como pasta do cliente.
const LIMPAVEIS: &[&str] = &[
    r"\windows\temp",
    r"\appdata\local\temp",
    r"\windows\softwaredistribution\download",
    r"\windows.old",
    r"\programdata\microsoft\windows\wer",
    r"\windows\servicing\logfiles",
    r"\windows\logs",
];

/// Compara caminho por COMPONENTE inteiro, e não por substring.
///
/// `contains` casa `\windows.old` dentro de
/// `...\downloads\windows.old-backup` — uma pasta que o cliente batizou com
/// nome parecido e que nada tem a ver com o backup de upgrade do Windows.
/// Oferecer apagar isso é o único erro deste produto que não tem desfazer, e
/// probabilidade baixa não compensa consequência irreversível. Comparando
/// componente inteiro contra componente inteiro, `windows.old-backup` nunca
/// casa com o padrão `windows.old`, porque são componentes diferentes — não
/// porque um é prefixo do outro.
fn contem_como_componente(caminho: &str, padrao: &str) -> bool {
    let componentes_caminho: Vec<&str> = caminho.split('\\').filter(|c| !c.is_empty()).collect();
    let componentes_padrao: Vec<&str> = padrao.split('\\').filter(|c| !c.is_empty()).collect();

    if componentes_padrao.is_empty() || componentes_padrao.len() > componentes_caminho.len() {
        return false;
    }

    componentes_caminho
        .windows(componentes_padrao.len())
        .any(|janela| janela == componentes_padrao.as_slice())
}

/// Decide o que uma pasta é. Pura, testável sem disco.
///
/// A ORDEM DOS TESTES IMPORTA, e o padrão é o seguro: o que não for
/// reconhecido como limpável é do cliente. Inverter isso ofereceria apagar
/// pasta desconhecida, e apagar arquivo de quem pagou não tem desfazer.
pub fn classificar(caminho: &str, leu: bool) -> Natureza {
    if !leu {
        return Natureza::NaoSei;
    }

    let minusculo = caminho.to_lowercase().replace('/', "\\");

    if LIMPAVEIS.iter().any(|p| contem_como_componente(&minusculo, p)) {
        return Natureza::PodeLimpar;
    }

    Natureza::Seu
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderMap {
    pub root: String,
    /// SOMA DAS RAÍZES VARRIDAS — não o tamanho do disco, nem o espaço usado
    /// nele. `mapear()` varre uma pasta só, e aqui o total é dela mesma; já
    /// `mapear_o_disco()` soma só as raízes de `raizes_do_disco()` (perfil do
    /// usuário, os dois `Program Files`, `ProgramData` e `C:\Windows`) — nem
    /// `System Volume Information`, nem perfil de outro usuário, nem o disco
    /// inteiro entram nessa conta. Apresentar isto como "seu disco tem X" na
    /// tela mentiria por baixo do valor real; a interface precisa dizer o que
    /// a varredura de fato cobriu.
    pub total_bytes: u64,
    pub total_formatted: String,
    pub folders: Vec<FolderEntry>,
    /// Quantas pastas não puderam ser lidas por falta de permissão.
    pub unreadable: usize,
    /// Verdadeiro quando o RELÓGIO estourou — e só isso. Não é sinônimo de
    /// "alguma pasta ficou parcial": `partial` também liga por limite de
    /// profundidade e por descendente sem permissão (ver `somar`), e as duas
    /// coisas são comuns em pasta grande e legítima. Antes desta correção
    /// este campo virava verdadeiro sempre que QUALQUER pasta esbarrava em
    /// permissão negada num canto qualquer — a mesma varredura que terminou
    /// dentro do prazo alegava para o cliente "a varredura não terminou
    /// dentro do tempo" (é o texto exato que `main.ts` mostra para este
    /// campo). Descendente ilegível já tem canal próprio — `unreadable` — e
    /// não precisa (nem pode) emprestar significado deste aqui.
    pub timed_out: bool,
}

pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;

    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.0} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Se o caminho é um link para outro lugar.
///
/// Seguir link conta o mesmo arquivo duas vezes e pode entrar em laço. O
/// atributo é lido dos metadados sem seguir o link — `symlink_metadata` é o
/// ponto central aqui, `metadata` iria atrás do destino e não veria a marca.
pub fn e_link(atributos: u32) -> bool {
    atributos & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

/// Estado compartilhado da varredura.
struct Varredura {
    ilegiveis: usize,
    prazo: std::time::Instant,
    /// Ligado assim que o prazo estoura. A partir daí toda soma volta na hora.
    estourou: bool,
}

impl Varredura {
    fn sem_tempo(&mut self) -> bool {
        if self.estourou {
            return true;
        }

        // A checagem é por pasta, não por arquivo: consultar o relógio a cada
        // arquivo custaria mais que ler o próprio arquivo.
        if std::time::Instant::now() >= self.prazo {
            self.estourou = true;
        }

        self.estourou
    }
}

/// Soma recursiva do conteúdo de uma pasta.
///
/// Devolve o total e se a soma foi cortada — por profundidade ou por tempo.
fn somar(dir: &Path, profundidade: u32, v: &mut Varredura) -> (u64, bool) {
    if profundidade >= PROFUNDIDADE_MAXIMA || v.sem_tempo() {
        return (0, true);
    }

    let Ok(entradas) = std::fs::read_dir(dir) else {
        // Pasta protegida. Comum e esperado; contamos para poder avisar que a
        // varredura não viu tudo, em vez de apresentar um total incompleto
        // como se fosse completo.
        //
        // Devolve `cortado = true` mesmo aqui embaixo na árvore, e não só por
        // profundidade ou tempo: o total do ancestral também virou piso, e
        // `partial` é o único canal que avisa isso à interface. Antes desta
        // correção o retorno era `false`, e um descendente sem permissão
        // desaparecia sem deixar rastro no `FolderEntry` do pai.
        v.ilegiveis += 1;
        return (0, true);
    };

    let mut total = 0u64;
    let mut cortado = false;

    for entrada in entradas.flatten() {
        // `entrada.metadata()`, e não `caminho.symlink_metadata()`: no Windows
        // o primeiro reaproveita o que a leitura da pasta já trouxe, enquanto o
        // segundo abre o arquivo de novo. É uma chamada de sistema a menos por
        // arquivo, e numa varredura de milhões de arquivos isso é a diferença
        // entre caber no prazo e não caber. Nenhum dos dois segue link.
        let Ok(meta) = entrada.metadata() else {
            continue;
        };

        if e_link(meta.file_attributes()) {
            continue;
        }

        if meta.is_dir() {
            let (bytes, parcial) = somar(&entrada.path(), profundidade + 1, v);
            total += bytes;
            cortado |= parcial;
        } else {
            total += meta.file_size();
        }
    }

    (total, cortado)
}

/// Se o NÍVEL 1 desta pasta foi lido com sucesso — e só isso.
///
/// DE PROPÓSITO não recebe a `Varredura` da soma recursiva. `v.ilegiveis`
/// conta permissão negada em QUALQUER profundidade da subárvore, e ligar essa
/// contagem a esta decisão foi o bug: numa máquina real, quase toda pasta
/// grande e legítima tem algum descendente inacessível — perfil de outro
/// usuário, cache de outro programa, uma pasta de sistema no meio. Uma pasta
/// de 150 GB medida com 99% de exatidão recebia o MESMO rótulo `NaoSei` que
/// uma pasta com zero bytes lidos.
///
/// Ilegibilidade de descendente já tem canal próprio — `partial`, que `somar`
/// agora devolve `true` para exatamente esse caso — e por isso não precisa
/// (nem pode) influenciar esta função. Chamar `read_dir` de novo aqui é uma
/// chamada de sistema a mais por pasta de nível 1 — dezenas por varredura,
/// não milhões — e é o preço de não confundir "não consegui olhar" com
/// "olhei quase tudo".
fn nivel1_foi_lido(caminho: &Path) -> bool {
    std::fs::read_dir(caminho).is_ok()
}

/// O que costuma ocupar espaço em cada pasta conhecida.
///
/// O nome sozinho não diz nada para quem não é técnico. "AppData" é a maior
/// pasta da maioria dos perfis e ninguém sabe o que tem lá dentro.
pub fn explicar(nome: &str) -> &'static str {
    match nome.to_lowercase().as_str() {
        "appdata" => {
            "Dados dos programas instalados: cache de navegador, e-mail baixado, \
             projetos de editor. É quase sempre a maior pasta do perfil, e quase \
             nada aí dentro pode ser apagado na mão sem quebrar programa."
        }
        "downloads" => {
            "Downloads. Costuma ser o ganho mais fácil e mais seguro: instalador \
             velho, ISO, zip já extraído. Confira antes de apagar."
        }
        "documents" | "documentos" => "Seus documentos. Não apague sem olhar.",
        "desktop" | "área de trabalho" => "Arquivos da Área de Trabalho.",
        "pictures" | "imagens" => "Fotos e imagens.",
        "videos" | "vídeos" => {
            "Vídeos. Junto com jogos, é o que mais come disco sem ninguém perceber."
        }
        "music" | "músicas" => "Músicas.",
        "onedrive" => {
            "Pasta do OneDrive. Arquivo marcado como \"sempre disponível\" ocupa \
             espaço aqui mesmo estando na nuvem — dá para liberar pelo próprio \
             OneDrive sem perder nada."
        }
        "saved games" => "Jogos salvos.",
        "steamlibrary" | "steam" => {
            "Biblioteca da Steam. Jogo instalado e não jogado há anos costuma ser \
             a maior economia possível — e reinstalar depois é só baixar de novo."
        }
        _ => "",
    }
}

/// Varre uma pasta e devolve os filhos de primeiro nível, do maior para o menor.
pub fn mapear(raiz: &Path, limite: usize) -> Result<FolderMap, String> {
    if !raiz.is_dir() {
        return Err(format!("`{}` não é uma pasta acessível.", raiz.display()));
    }

    let entradas =
        std::fs::read_dir(raiz).map_err(|e| format!("Não foi possível ler `{}`: {}", raiz.display(), e))?;

    // A lista inteira é levantada antes de somar qualquer coisa: é ela que diz
    // entre quantas pastas o prazo precisa ser dividido.
    let mut filhos: Vec<PathBuf> = Vec::new();
    let mut total = 0u64;

    for entrada in entradas.flatten() {
        let caminho = entrada.path();

        let Ok(meta) = caminho.symlink_metadata() else {
            continue;
        };

        if e_link(meta.file_attributes()) {
            continue;
        }

        // Arquivo solto na raiz entra no total, mas não vira linha do mapa: o
        // mapa é de pastas.
        if meta.is_dir() {
            filhos.push(caminho);
        } else {
            total += meta.file_size();
        }
    }

    let fim = std::time::Instant::now() + std::time::Duration::from_secs(ORCAMENTO_SEGUNDOS);
    let mut ilegiveis = 0usize;
    // SÓ o relógio pode ligar isto — não `partial`. `partial` também vira
    // `true` por limite de profundidade e por descendente sem permissão
    // (ver `somar`), e as duas coisas acontecem em pasta grande e legítima
    // sem que a varredura tenha estourado o prazo. `Varredura::estourou` só
    // liga dentro de `sem_tempo`, quando o relógio de verdade estourou — e
    // o corte por profundidade e o corte por `read_dir` que falha retornam
    // ANTES de chamar `sem_tempo` (curto-circuito do `||` em `somar`), então
    // nenhum dos dois toca `estourou`. É por isso que agregar por aqui, e
    // não por `partial`, mantém este campo dizendo só o que o nome promete.
    let mut algum_prazo_estourou = false;
    let mut pastas: Vec<FolderEntry> = Vec::new();

    for (indice, caminho) in filhos.iter().enumerate() {
        let restante = fim.saturating_duration_since(std::time::Instant::now());
        let minha_fatia = fatia(restante, filhos.len() - indice);

        let mut v = Varredura {
            ilegiveis: 0,
            prazo: std::time::Instant::now() + minha_fatia,
            estourou: false,
        };

        let (bytes, partial) = somar(caminho, 1, &mut v);
        let leu_com_sucesso = nivel1_foi_lido(caminho);

        total += bytes;
        ilegiveis += v.ilegiveis;
        algum_prazo_estourou |= v.estourou;

        let name = caminho
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let caminho_texto = caminho.to_string_lossy().to_string();

        pastas.push(FolderEntry {
            explanation: explicar(&name).to_string(),
            formatted: format_size(bytes),
            natureza: classificar(&caminho_texto, leu_com_sucesso),
            path: caminho_texto,
            percent: 0.0,
            name,
            bytes,
            partial,
        });
    }

    // Pasta cortada vem antes de pasta medida por inteiro, e não pelo número.
    //
    // Comparar os dois pelo tamanho é comparar coisas diferentes: o de uma é
    // total, o da outra é piso. Nesta máquina isso colocava `Videos`, com 3,4 GB
    // medidos até o fim, acima de `AppData`, que tem 151 GB e só deu tempo de
    // contar 2,5 GB — ou seja, o mapa apontava para a pasta errada, que é a
    // única coisa que ele tem que acertar.
    //
    // Quem não terminou é, por definição, grande demais para caber no prazo. É
    // o candidato mais provável a ser o sumidouro, e a interface diz "pelo
    // menos" em cima do número para ninguém tomar o piso por total.
    pastas.sort_by(|a, b| {
        b.partial
            .cmp(&a.partial)
            .then_with(|| b.bytes.cmp(&a.bytes))
    });
    pastas.truncate(limite);

    // A porcentagem é do total varrido, e por isso só pode ser calculada
    // depois de somar tudo.
    if total > 0 {
        for pasta in &mut pastas {
            pasta.percent = pasta.bytes as f64 / total as f64 * 100.0;
        }
    }

    Ok(FolderMap {
        root: raiz.to_string_lossy().to_string(),
        total_bytes: total,
        total_formatted: format_size(total),
        folders: pastas,
        unreadable: ilegiveis,
        timed_out: algum_prazo_estourou,
    })
}

/// A pasta do usuário: onde está o espaço que ele mesmo pode decidir sobre.
pub fn perfil_do_usuario() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Users"))
}

/// As pastas de primeiro nível que valem varrer, na ordem em que costumam
/// pesar.
///
/// POR QUE UMA LISTA E NÃO `C:\` INTEIRO. Varrer a raiz do disco entra em
/// `System Volume Information` e no `$Recycle.Bin` de outros usuários — pastas
/// que devolvem erro de permissão e não acrescentam nada. A lista cobre onde o
/// espaço realmente vai, medido na máquina que motivou este trabalho:
/// AppData 176 GB, Steam 122 GB, Downloads 48 GB, Windows 29 GB.
///
/// Só entra o que existe: máquina sem `Program Files (x86)` (Windows ARM, por
/// exemplo) simplesmente não tem essa linha, em vez de mostrar uma pasta vazia.
pub fn raizes_do_disco() -> Vec<PathBuf> {
    let candidatos = [
        std::env::var("ProgramFiles").ok(),
        std::env::var("ProgramFiles(x86)").ok(),
        std::env::var("ProgramData").ok(),
        std::env::var("SystemRoot").ok(),
        Some(perfil_do_usuario().to_string_lossy().to_string()),
    ];

    let existentes: Vec<PathBuf> = candidatos
        .into_iter()
        .flatten()
        .map(PathBuf::from)
        .filter(|caminho| caminho.exists())
        .collect();

    deduplicar_por_caminho_normalizado(existentes)
}

/// DEDUPLICAÇÃO POR TEXTO NORMALIZADO, isolada como função pura para dar para
/// testar o mecanismo em si — e não só o acaso de `raizes_do_disco()` nesta
/// máquina, onde `ProgramFiles` e `ProgramFiles(x86)` já são strings
/// diferentes e um teste que chama só `raizes_do_disco()` passaria mesmo com
/// este bloco inteiro apagado.
///
/// Em alguns Windows, `ProgramFiles` e `ProgramFiles(x86)` apontam para o
/// mesmo lugar — somar as duas passaria o total do tamanho do disco. Mantém
/// a PRIMEIRA ocorrência de cada caminho, na ordem de entrada, porque a lista
/// chega em ordem de peso (ver `raizes_do_disco`) e é essa ordem que decide
/// qual nome sobrevive.
fn deduplicar_por_caminho_normalizado(caminhos: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut vistos: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut resultado = Vec::new();

    for caminho in caminhos {
        let chave = caminho.to_string_lossy().to_lowercase();

        if vistos.insert(chave) {
            resultado.push(caminho);
        }
    }

    resultado
}

/// O mapa do disco: uma linha por raiz, sem descer nelas.
///
/// DUAS CAMADAS, E ESTA É A PRIMEIRA. Varrer 476 GB a fundo leva minutos —
/// medido: sete, na máquina onde `mapear` foi escrito. Ninguém espera olhando
/// tela parada. Aqui cada raiz vira uma linha; o detalhe só é varrido quando o
/// cliente clica numa delas, chamando `mapear` como já se faz hoje.
pub fn mapear_o_disco(limite: usize) -> Result<FolderMap, String> {
    mapear_o_disco_com_raizes(raizes_do_disco(), limite)
}

/// Mesma lógica de `mapear_o_disco`, mas recebendo as raízes de fora.
///
/// SEPARADA SÓ PARA DAR PARA TESTAR O RAMO `Err(_)` ABAIXO. Esse ramo é a
/// regra que motivou a tarefa inteira: raiz ilegível vira `NaoSei` no mapa,
/// e não desaparece dele. `mapear_o_disco()` sempre chama `raizes_do_disco()`,
/// que só devolve caminho que `existe()` — ou seja, nunca produz uma raiz que
/// falhe em `mapear()` por não ser pasta, e o ramo `Err(_)` fica sem nenhum
/// teste capaz de reprovar se alguém trocar "vira NaoSei" por "pula a raiz".
/// Passando uma raiz inventada aqui, o teste aciona o `Err` de verdade sem
/// precisar de ACL nenhuma.
fn mapear_o_disco_com_raizes(raizes: Vec<PathBuf>, limite: usize) -> Result<FolderMap, String> {
    let mut folders: Vec<FolderEntry> = Vec::new();
    let mut ilegiveis = 0usize;
    let mut estourou = false;

    for raiz in raizes {
        match mapear(&raiz, limite) {
            Ok(parcial) => {
                ilegiveis += parcial.unreadable;
                estourou = estourou || parcial.timed_out;

                let nome = raiz
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| raiz.to_string_lossy().to_string());
                let caminho = raiz.to_string_lossy().to_string();

                folders.push(FolderEntry {
                    natureza: classificar(&caminho, true),
                    path: caminho,
                    bytes: parcial.total_bytes,
                    formatted: format_size(parcial.total_bytes),
                    // Preenchido no fim, quando o total de todas for conhecido.
                    percent: 0.0,
                    // `nome`, e não o caminho inteiro: `explicar` casa por
                    // igualdade exata contra chaves curtas como "steam" e
                    // "appdata". Passar `c:\program files (x86)` nunca bate
                    // com nada, e a explicação sai sempre vazia em silêncio —
                    // era esse o bug aqui antes desta correção.
                    explanation: explicar(&nome).to_string(),
                    partial: parcial.timed_out,
                    name: nome,
                });
            }
            Err(_) => {
                // A RAIZ QUE NÃO DEU PARA LER APARECE MESMO ASSIM, como
                // `NaoSei`. Omiti-la faria o total mentir por baixo, e o
                // cliente veria o espaço sumir no nada.
                let caminho = raiz.to_string_lossy().to_string();
                ilegiveis += 1;

                folders.push(FolderEntry {
                    name: raiz
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| caminho.clone()),
                    natureza: Natureza::NaoSei,
                    path: caminho,
                    bytes: 0,
                    formatted: format_size(0),
                    percent: 0.0,
                    explanation: String::new(),
                    partial: true,
                });
            }
        }
    }

    let total: u64 = folders.iter().map(|f| f.bytes).sum();

    for pasta in &mut folders {
        pasta.percent = if total == 0 {
            0.0
        } else {
            (pasta.bytes as f64 / total as f64) * 100.0
        };
    }

    folders.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    Ok(FolderMap {
        root: "C:\\".to_string(),
        total_bytes: total,
        total_formatted: format_size(total),
        folders,
        unreadable: ilegiveis,
        timed_out: estourou,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UMA LINHA DE ATRIBUTO É O CONTRATO INTEIRO COM A TELA.
    ///
    /// Tirar o `#[serde(tag = "tipo")]` do `Natureza` não quebra nada aqui:
    /// `cargo test` passa, `tsc` passa — e o enum vira a string `"NaoSei"`, com
    /// `linha.natureza.tipo` virando `undefined` na interface. Aí a pasta que
    /// não deu para ler passa a ser tratada como qualquer outra, que é
    /// exatamente o que estes três estados existem para impedir.
    ///
    /// O mesmo teste existe para o `Medida` do `diskspace.rs`, pela mesma razão
    /// e depois do mesmo susto.
    #[test]
    fn a_natureza_chega_na_tela_como_objeto_com_campo_tipo() {
        for (natureza, esperado) in [
            (Natureza::PodeLimpar, r#"{"tipo":"PodeLimpar"}"#),
            (Natureza::Seu, r#"{"tipo":"Seu"}"#),
            (Natureza::NaoSei, r#"{"tipo":"NaoSei"}"#),
        ] {
            assert_eq!(
                serde_json::to_string(&natureza).expect("Natureza serializa"),
                esperado,
                "{:?} não chega à tela com `tipo`",
                natureza
            );
        }
    }

    #[test]
    fn tamanho_sai_na_unidade_certa() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(2048), "2 KB");
        assert_eq!(format_size(5 * 1024 * 1024), "5 MB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    #[test]
    fn atributo_de_link_e_reconhecido() {
        const DIRETORIO: u32 = 0x10;

        assert!(e_link(FILE_ATTRIBUTE_REPARSE_POINT));
        // Junction é diretório E ponto de nova análise ao mesmo tempo.
        assert!(e_link(DIRETORIO | FILE_ATTRIBUTE_REPARSE_POINT));
        assert!(!e_link(DIRETORIO));
        assert!(!e_link(0));
    }

    #[test]
    fn pastas_conhecidas_sao_explicadas_em_portugues() {
        // AppData é a maior pasta da maioria dos perfis e a que ninguém entende.
        assert!(explicar("AppData").contains("cache"));
        // E a explicação precisa avisar que não é para sair apagando.
        assert!(explicar("AppData").contains("quebrar programa"));

        assert!(explicar("Downloads").contains("instalador"));
        // Maiúscula e minúscula não podem mudar a resposta.
        assert_eq!(explicar("downloads"), explicar("DOWNLOADS"));

        // Pasta criada pelo usuário não recebe explicação inventada.
        assert_eq!(explicar("Projetos do Cliente"), "");
    }

    #[test]
    fn varredura_respeita_o_prazo() {
        // O requisito real: o botão não pode parecer travado. Sem prazo, esta
        // mesma varredura levou sete minutos.
        let inicio = std::time::Instant::now();
        let _ = mapear(&perfil_do_usuario(), 10).expect("o perfil precisa ser legível");
        let gasto = inicio.elapsed().as_secs();

        println!("varredura levou {} s", gasto);
        assert!(
            gasto <= ORCAMENTO_SEGUNDOS + 10,
            "a varredura levou {} s, muito além do prazo de {} s",
            gasto,
            ORCAMENTO_SEGUNDOS
        );
    }

    #[test]
    fn mapeia_o_perfil_desta_maquina() {
        let mapa = mapear(&perfil_do_usuario(), 10).expect("o perfil precisa ser legível");

        println!("{} — total {}{}", mapa.root, mapa.total_formatted,
                 if mapa.timed_out { " (parou no prazo)" } else { "" });
        println!("{} pastas sem permissão de leitura", mapa.unreadable);
        for f in &mapa.folders {
            println!(
                "  {:>9}  {:>5.1}%  {}{}",
                f.formatted,
                f.percent,
                f.name,
                if f.partial { " (parcial)" } else { "" }
            );
        }

        assert!(!mapa.folders.is_empty(), "todo perfil tem subpastas");
        assert!(mapa.folders.len() <= 10, "o limite precisa ser respeitado");

        // As cortadas primeiro, e dentro de cada grupo do maior para o menor.
        // Ordenar tudo junto pelo número compara total com piso e aponta a
        // pasta errada — foi o que aconteceu antes desta regra existir.
        let chave = |f: &FolderEntry| (!f.partial, std::cmp::Reverse(f.bytes));
        assert!(
            mapa.folders.windows(2).all(|p| chave(&p[0]) <= chave(&p[1])),
            "ordem quebrada"
        );

        // Nenhuma pasta pode ocupar mais que o total, e a soma das partes não
        // pode passar do todo — os dois sintomas de link contado duas vezes.
        let soma: u64 = mapa.folders.iter().map(|f| f.bytes).sum();
        assert!(
            soma <= mapa.total_bytes,
            "as pastas somam {} num total de {} — link contado em dobro",
            soma,
            mapa.total_bytes
        );

        for f in &mapa.folders {
            assert!(f.percent >= 0.0 && f.percent <= 100.0, "{}%", f.percent);
        }
    }

    #[test]
    fn caminho_que_nao_e_pasta_e_recusado() {
        let erro = mapear(Path::new("C:\\Windows\\explorer.exe"), 5);
        assert!(erro.is_err());

        let inexistente = mapear(Path::new("C:\\pasta que nao existe 12345"), 5);
        assert!(inexistente.is_err());
    }

    #[test]
    fn pasta_que_nao_deu_para_ler_nunca_e_zero_nem_limpavel() {
        // "NAO SEI" E UM TERCEIRO ESTADO, e nao um zero.
        //
        // Uma pasta sem permissao aparecendo como 0 GB faria o total mentir, e o
        // cliente concluiria que o espaco sumiu no nada. E marca-la como
        // limpavel seria oferecer apagar o que nem foi possivel olhar.
        assert_eq!(classificar(r"C:\Windows\System32\config", false), Natureza::NaoSei);
    }

    #[test]
    fn arquivo_do_cliente_nunca_e_marcado_como_limpavel() {
        // O PIOR ERRO POSSIVEL DESTE PROGRAMA. Apagar jogo ou download de quem
        // pagou nao tem desfazer. Steam, Downloads e Documentos sao DELE.
        for caminho in [
            r"C:\Program Files (x86)\Steam",
            r"C:\Users\Fulano\Downloads",
            r"C:\Users\Fulano\Documents",
            r"C:\Users\Fulano\Videos",
        ] {
            assert_eq!(
                classificar(caminho, true),
                Natureza::Seu,
                "{} foi marcado como limpavel", caminho
            );
        }
    }

    #[test]
    fn categoria_conhecida_de_limpeza_e_marcada_como_limpavel() {
        // O que o `diskspace.rs` ja sabe limpar continua limpavel aqui, para as
        // duas telas nao discordarem uma da outra.
        assert_eq!(classificar(r"C:\Windows\Temp", true), Natureza::PodeLimpar);
        assert_eq!(
            classificar(r"C:\Windows\SoftwareDistribution\Download", true),
            Natureza::PodeLimpar
        );
    }

    #[test]
    fn na_duvida_e_do_cliente_e_nao_limpavel() {
        // CANARIO. Uma pasta que ninguem reconhece nao pode cair em `PodeLimpar`
        // por descuido de ordem dos `if`. O padrao seguro e "e do cliente".
        assert_eq!(classificar(r"C:\MinhaPastaEstranha", true), Natureza::Seu);
    }

    #[test]
    fn casamento_e_por_componente_e_nao_por_substring() {
        // `\windows.old` E UM COMPONENTE INTEIRO DE CAMINHO, nao um prefixo de
        // texto. `contains` casava dentro de "windows.old-backup", uma pasta
        // que o cliente pode ter criado nos proprios Downloads sem nenhuma
        // relacao com o backup de upgrade do Windows. Oferecer apagar isso e
        // o unico erro deste produto sem desfazer.
        assert_eq!(classificar(r"C:\Windows.old", true), Natureza::PodeLimpar);
        assert_eq!(
            classificar(r"C:\Users\Fulano\Downloads\windows.old-backup", true),
            Natureza::Seu,
            "windows.old-backup nao e o mesmo componente que windows.old"
        );
        assert_eq!(
            classificar(r"C:\Users\Fulano\Documents\meus-windows-logs", true),
            Natureza::Seu,
            "meus-windows-logs nao pode casar com o padrao \\windows\\logs"
        );
    }

    #[test]
    fn read_dir_que_falha_em_qualquer_profundidade_vira_parcial() {
        // Antes da correção, `somar` devolvia `(0, false)` quando o próprio
        // `read_dir` falhava — ou seja, permissão negada num descendente
        // desaparecia sem deixar rastro em `partial`, o único sinal que a
        // interface tem de "isto é piso, não total". Um caminho que não
        // existe entra pelo MESMO ramo de código que uma pasta sem permissão
        // (`std::fs::read_dir` devolvendo `Err`), de um jeito determinístico
        // que não depende de ACL de disco — ver o teste seguinte para o
        // porquê isso importa neste ambiente específico.
        let mut v = Varredura {
            ilegiveis: 0,
            prazo: std::time::Instant::now() + std::time::Duration::from_secs(30),
            estourou: false,
        };
        let caminho_inexistente =
            std::env::temp_dir().join("otimiza_teste_caminho_que_nao_existe_12345");
        let _ = std::fs::remove_dir_all(&caminho_inexistente);

        let (bytes, cortado) = somar(&caminho_inexistente, 2, &mut v);

        assert_eq!(bytes, 0);
        assert!(cortado, "read_dir que falha precisa marcar `partial`, não desaparecer");
        assert_eq!(v.ilegiveis, 1);
    }

    #[test]
    fn nivel1_legivel_nao_depende_de_descendente_ilegivel() {
        // O NÚCLEO DO BUG. A decisão "consegui ler esta pasta" (usada para
        // `Natureza`) não pode se basear em `v.ilegiveis`, que soma permissão
        // negada em QUALQUER profundidade da subárvore. `nivel1_foi_lido` nem
        // recebe `Varredura` como parâmetro — é estruturalmente impossível
        // dela reagir a um descendente ilegível. Aqui simulamos exatamente o
        // cenário do achado: uma pasta cujo próprio `read_dir` funciona, mas
        // cuja soma recursiva acumulou `ilegiveis > 0` por causa de
        // descendentes protegidos (perfil de outro usuário, cache de outro
        // programa — o caso comum numa máquina real).
        let pasta = std::env::temp_dir().join("otimiza_teste_nivel1_legivel");
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("criar pasta de teste");

        // `v.ilegiveis` alto de propósito — se `nivel1_foi_lido` consultasse
        // isto, a asserção abaixo reprovaria. Ela não consulta.
        let v = Varredura {
            ilegiveis: 3,
            prazo: std::time::Instant::now() + std::time::Duration::from_secs(30),
            estourou: false,
        };

        let leu = nivel1_foi_lido(&pasta);
        assert!(
            leu,
            "read_dir do próprio nível 1 funcionou; a ilegibilidade é só dos descendentes"
        );
        assert_ne!(
            classificar(&pasta.to_string_lossy(), leu),
            Natureza::NaoSei,
            "descendente ilegível não pode reclassificar a pasta inteira como NaoSei"
        );

        drop(v);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn nivel1_ilegivel_e_reconhecido_como_tal() {
        // O OUTRO LADO do teste acima: `nivel1_foi_lido` não pode virar um
        // `true` incondicional. Se a própria pasta de nível 1 não abre, isto
        // precisa devolver `false` — é o sinal que vira `NaoSei` em
        // `classificar`. Sem este teste, uma implementação que sempre
        // devolve `true` (esvaziando a checagem) passaria despercebida por
        // todos os outros testes deste arquivo.
        let inexistente =
            std::env::temp_dir().join("otimiza_teste_nivel1_que_nao_existe_98765");
        let _ = std::fs::remove_dir_all(&inexistente);

        assert!(!nivel1_foi_lido(&inexistente));
    }

    // O teste de integração via ACL real (`icacls /deny` + `mapear` de
    // ponta a ponta) que existia nesta rodada anterior foi REMOVIDO.
    //
    // Ele se autopulava neste sandbox de desenvolvimento por causa de
    // `SeBackupPrivilege` (confirmado com `whoami /priv`), e a rerevisão
    // confirmou que o mesmo motivo se aplica ao CI real deste repositório:
    // `.github/workflows/release.yml` roda em `runs-on: windows-latest`, e o
    // runner do GitHub executa o job com uma conta administrativa — a mesma
    // classe de ambiente que ignora a checagem de ACL ao abrir diretório com
    // semântica de backup. Ou seja: o teste não rodaria de verdade nem lá.
    // Um teste com nome de guarda que nunca reprova em lugar nenhum é pior
    // que não ter teste — ninguém olha de novo o que já parece coberto.
    //
    // A guarda que fica é a de baixo: um canário de texto-fonte, que não
    // depende de ACL, privilégio de processo, ou sistema operacional. Ela
    // não prova que o Windows de verdade nega a leitura — isso já está
    // provado pelos dois testes deterministas acima
    // (`nivel1_legivel_nao_depende_de_descendente_ilegivel` e
    // `nivel1_ilegivel_e_reconhecido_como_tal`, que testam cada metade do
    // mecanismo isoladamente). O que faltava, e é o que este canário fecha,
    // é a FIAÇÃO: que `mapear` de fato liga as duas metades chamando
    // `nivel1_foi_lido(caminho)` — e não voltou a usar `v.ilegiveis == 0`,
    // que é o bug original.
    #[test]
    fn mapear_liga_leu_com_sucesso_a_nivel1_foi_lido_e_nao_a_ilegiveis() {
        let fonte = codigo_fonte_deste_arquivo();

        // A mutação exata que o bug original era, e que a rerevisão
        // reintroduziu para provar que o teste de ACL não pegava nada:
        // `mapear:380` voltando a ler `v.ilegiveis == 0` em vez de chamar
        // `nivel1_foi_lido`. Se essa string aparecer nesta função, é o bug
        // de volta, ponto.
        assert!(
            !fonte.contains("let leu_com_sucesso = v.ilegiveis"),
            "mapear voltou a decidir `leu_com_sucesso` por `v.ilegiveis`, que soma \
             permissão negada em QUALQUER profundidade — exatamente o bug que \
             `nivel1_foi_lido` existe para evitar"
        );

        // E a fiação certa precisa estar presente — não basta a errada estar
        // ausente; alguém poderia trocar por uma terceira coisa igualmente
        // errada (um `true` incondicional, por exemplo) e as duas guardas
        // acima não pegariam isso sozinhas.
        assert!(
            fonte.contains("let leu_com_sucesso = nivel1_foi_lido(caminho);"),
            "mapear precisa decidir `leu_com_sucesso` chamando `nivel1_foi_lido(caminho)`"
        );
    }

    /// Lê o CÓDIGO deste próprio arquivo — só a parte de fora de `mod tests`
    /// — para os canários de texto-fonte deste módulo.
    ///
    /// Cortar em `#[cfg(test)]` não é cosmético: os próprios canários abaixo
    /// citam, em string literal, os padrões que reprovam (ex.:
    /// `"let leu_com_sucesso = v.ilegiveis"`). Sem o corte, um `.contains`
    /// rodando sobre o arquivo inteiro acharia essa string DENTRO do próprio
    /// teste que a procura, e a guarda reprovaria sempre — inclusive sem
    /// nenhuma mutação. `CARGO_MANIFEST_DIR` aponta sempre para `src-tauri`,
    /// independente de onde `cargo test` é chamado — o mesmo truque que a
    /// guarda de prosa em `commands.rs` usa para achar `main.ts`.
    fn codigo_fonte_deste_arquivo() -> String {
        let caminho = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("modules")
            .join("windows")
            .join("foldermap.rs");
        let fonte = std::fs::read_to_string(&caminho)
            .unwrap_or_else(|e| panic!("não consegui ler {:?}: {}", caminho, e));
        fonte
            .split_once("#[cfg(test)]")
            .map(|(codigo, _testes)| codigo.to_string())
            .unwrap_or(fonte)
    }

    #[test]
    fn permissao_negada_no_descendente_nao_liga_o_relogio() {
        // O NÚCLEO DO ACHADO 2. `partial` liga por três motivos (profundidade,
        // prazo, permissão), mas só um deles é o RELÓGIO de verdade —
        // `Varredura::estourou`. O ramo de `read_dir` que falha em `somar`
        // retorna `(0, true)` (marca `partial`) SEM jamais chamar
        // `sem_tempo`, então `estourou` tem que continuar `false`. Se um dia
        // alguém "simplificar" e fizer o ramo de permissão também acionar
        // `sem_tempo`/`estourou`, esta asserção reprova.
        let mut v = Varredura {
            ilegiveis: 0,
            prazo: std::time::Instant::now() + std::time::Duration::from_secs(30),
            estourou: false,
        };
        let caminho_inexistente = std::env::temp_dir()
            .join("otimiza_teste_permissao_nao_e_relogio_12345");
        let _ = std::fs::remove_dir_all(&caminho_inexistente);

        let (_, cortado) = somar(&caminho_inexistente, 2, &mut v);

        assert!(cortado, "read_dir que falha precisa marcar partial");
        assert!(
            !v.estourou,
            "descendente sem permissão não pode ligar o relógio — só o prazo de verdade liga isso"
        );
    }

    #[test]
    fn varredura_dentro_do_prazo_com_corte_por_profundidade_nao_e_timed_out() {
        // PONTA A PONTA, via `mapear`, e determinístico: sem ACL, sem
        // privilégio de processo, sem depender do relógio de verdade
        // estourar. Uma árvore mais funda que `PROFUNDIDADE_MAXIMA` corta por
        // profundidade (`partial = true` na pasta), o mesmo ramo de código
        // que o corte por permissão em `somar` — os dois retornam `(0/soma,
        // true)` sem passar por `sem_tempo`. Antes desta correção, `mapear`
        // fazia `timed_out: algum_cortado` (`algum_cortado |= partial`), e
        // este cenário — que termina bem dentro do orçamento de
        // `ORCAMENTO_SEGUNDOS` — teria acusado `timed_out: true` na cara do
        // cliente por uma varredura que nunca chegou perto do prazo.
        let raiz = std::env::temp_dir().join("otimiza_teste_timed_out_nao_e_profundidade");
        let _ = std::fs::remove_dir_all(&raiz);
        let mut fundo = raiz.join("pasta_funda");
        for i in 0..(PROFUNDIDADE_MAXIMA as usize + 10) {
            fundo = fundo.join(format!("nivel{}", i));
        }
        std::fs::create_dir_all(&fundo).expect("criar árvore funda de teste");

        let inicio = std::time::Instant::now();
        let mapa = mapear(&raiz, 5).expect("a raiz do teste é legível");
        let gasto = inicio.elapsed().as_secs();

        let _ = std::fs::remove_dir_all(&raiz);

        // A própria premissa do teste: terminou bem dentro do prazo.
        assert!(
            gasto < ORCAMENTO_SEGUNDOS,
            "o teste levou {} s — não serve para provar 'terminou dentro do prazo'",
            gasto
        );

        let entrada = mapa
            .folders
            .iter()
            .find(|f| f.name == "pasta_funda")
            .expect("pasta_funda precisa aparecer no mapa");
        assert!(
            entrada.partial,
            "a árvore é mais funda que PROFUNDIDADE_MAXIMA; a pasta tem que ficar parcial"
        );

        assert!(
            !mapa.timed_out,
            "corte por profundidade não é corte por prazo — timed_out mentiu de novo"
        );
    }

    #[test]
    fn mapear_liga_timed_out_a_estourou_e_nao_a_partial() {
        // Canário de texto-fonte para a mesma fiação, do jeito que o achado 1
        // já fez para `leu_com_sucesso`: prova que `mapear` monta `timed_out`
        // a partir do relógio (`v.estourou`), e não do que a rodada anterior
        // usava (`algum_cortado |= partial` / `timed_out: algum_cortado`).
        // Sozinho, `varredura_dentro_do_prazo_com_corte_por_profundidade_nao_e_timed_out`
        // já reprova se a fiação regredir — este teste só torna o motivo
        // explícito sem precisar montar disco.
        let fonte = codigo_fonte_deste_arquivo();

        assert!(
            !fonte.contains("algum_cortado |= partial"),
            "mapear voltou a agregar `partial` (profundidade + prazo + permissão, os \
             três misturados) para decidir timed_out — é exatamente a diluição que \
             este achado corrigiu"
        );
        assert!(
            !fonte.contains("timed_out: algum_cortado"),
            "timed_out voltou a vir de `algum_cortado`, que inclui corte por permissão"
        );
        assert!(
            fonte.contains("algum_prazo_estourou |= v.estourou"),
            "timed_out precisa vir só de `Varredura::estourou`, o único sinal que é \
             de fato o relógio"
        );
    }

    #[test]
    fn pasta_vazia_nao_gera_divisao_por_zero() {
        let temporaria = std::env::temp_dir().join("otimiza_teste_mapa_vazio");
        std::fs::create_dir_all(&temporaria).expect("criar pasta de teste");

        let mapa = mapear(&temporaria, 5).expect("pasta vazia é legível");

        assert_eq!(mapa.total_bytes, 0);
        assert!(mapa.folders.is_empty());

        let _ = std::fs::remove_dir(&temporaria);
    }

    #[test]
    fn as_raizes_incluem_onde_os_jogos_ficam() {
        // O FURO QUE ESTE PLANO EXISTE PARA FECHAR.
        //
        // Medido na maquina do dono: 122 GB de Steam em `Program Files (x86)`,
        // contra 176 GB em AppData. O mapa so varria o perfil do usuario, entao
        // era cego para a SEGUNDA MAIOR coisa da maquina -- num produto cujo
        // publico inteiro e jogador.
        let raizes: Vec<String> = raizes_do_disco()
            .iter()
            .map(|p| p.to_string_lossy().to_lowercase())
            .collect();

        let juntas = raizes.join(" | ");

        assert!(juntas.contains("program files (x86)"), "faltou: {}", juntas);
        assert!(juntas.contains("program files"), "faltou: {}", juntas);
        assert!(juntas.contains("users") || juntas.contains("usuários"), "faltou: {}", juntas);
    }

    #[test]
    fn nenhuma_raiz_repetida() {
        // `Program Files` e prefixo de `Program Files (x86)` -- montar a lista com
        // um `contains` descuidado somaria a mesma pasta duas vezes, e o total do
        // mapa passaria do tamanho do disco.
        let raizes = raizes_do_disco();
        let mut vistos = std::collections::BTreeSet::new();

        for r in &raizes {
            assert!(
                vistos.insert(r.to_string_lossy().to_lowercase()),
                "raiz repetida: {:?}", r
            );
        }
    }

    #[test]
    fn deduplicar_por_caminho_normalizado_remove_a_repetida() {
        // O ACHADO DA RODADA ANTERIOR: `nenhuma_raiz_repetida` passava mesmo
        // com o bloco de dedup inteiro removido, porque nesta máquina
        // `ProgramFiles` e `ProgramFiles(x86)` já são strings diferentes --
        // o teste constatava um acaso do ambiente, não o mecanismo. Chamando
        // a função pura direto, com dois candidatos que só diferem em
        // maiúscula/minúscula, o mecanismo fica exposto de verdade: tirar a
        // deduplicação (por exemplo devolvendo `caminhos` sem passar pelo
        // `BTreeSet`) faz este teste reprovar, não só o de cima.
        let candidatos = vec![
            PathBuf::from(r"C:\Program Files"),
            PathBuf::from(r"c:\program files"),
            PathBuf::from(r"C:\Program Files (x86)"),
        ];

        let resultado = deduplicar_por_caminho_normalizado(candidatos);

        assert_eq!(resultado.len(), 2, "esperava 2 raizes unicas, veio {:?}", resultado);
        assert_eq!(resultado[0], PathBuf::from(r"C:\Program Files"),
            "a PRIMEIRA ocorrencia deve sobreviver, e nao a ultima");
    }

    #[test]
    fn raiz_ilegivel_aparece_como_naosei_e_nao_e_pulada() {
        // A REGRA QUE MOTIVOU A TAREFA, agora com guarda de verdade. Raiz
        // ilegivel pulada em vez de virar `NaoSei` faz o total do disco
        // mentir por baixo -- exatamente o sintoma que o projeto existe para
        // resolver, só que ao contrário (some espaço, em vez de aparecer a
        // mais). Uma raiz que simplesmente não existe já basta para acionar
        // o `Err(_)` de `mapear()` (falha em `raiz.is_dir()`), sem precisar
        // de ACL nenhuma -- ACL de verdade se autopula no CI, como a rodada
        // anterior já mostrou com o teste de `icacls` removido.
        let raiz_inexistente = std::env::temp_dir().join("otimiza_raiz_que_nao_existe_de_verdade_xyz789");
        assert!(!raiz_inexistente.exists(), "a raiz de teste precisa MESMO nao existir");

        let mapa = mapear_o_disco_com_raizes(vec![raiz_inexistente.clone()], 12)
            .expect("raiz ilegivel nao pode derrubar o mapa do disco inteiro");

        // "Pular a raiz" faria isto aqui virar 0 -- é exatamente esse
        // mutante que este teste tem que pegar.
        assert_eq!(mapa.folders.len(), 1, "a raiz ilegivel sumiu do mapa em vez de virar NaoSei");
        assert_eq!(mapa.folders[0].natureza, Natureza::NaoSei);
        assert_eq!(mapa.folders[0].bytes, 0);
        assert!(mapa.folders[0].partial);
        assert_eq!(mapa.unreadable, 1, "raiz ilegivel precisa contar em unreadable");
    }

    #[test]
    fn explicar_recebe_o_nome_e_nao_o_caminho_inteiro() {
        // O BUG: `mapear_o_disco` chamava `explicar(&raiz.to_string_lossy())`
        // -- caminho inteiro, tipo `c:\program files (x86)` -- contra uma
        // função que casa por IGUALDADE EXATA com chaves curtas ("steam",
        // "appdata"). Caminho completo nunca bate com nada, e a explicação
        // saia sempre vazia em silêncio. `mapear_o_disco_com_raizes` usando
        // uma raiz cujo nome de arquivo é "steam" precisa devolver a
        // explicação de verdade -- vazio aqui reprova o mutante que volta a
        // passar o caminho inteiro.
        // O NOME DO ARQUIVO precisa ser exatamente "steam" -- é contra isso
        // que `explicar` casa por igualdade exata. Uma pasta de teste com
        // outro nome não pegaria o bug: precisa ser o último componente do
        // caminho, não o caminho todo, a bater com a chave curta.
        let raiz_steam = std::env::temp_dir().join("otimiza_teste_explicar").join("steam");
        std::fs::create_dir_all(&raiz_steam).expect("criar pasta de teste");

        let mapa = mapear_o_disco_com_raizes(vec![raiz_steam.clone()], 12)
            .expect("pasta de teste e legivel");

        assert_eq!(mapa.folders.len(), 1);
        assert!(
            !mapa.folders[0].explanation.is_empty(),
            "explicacao saiu vazia -- explicar() recebeu o caminho inteiro, e nao o nome"
        );

        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("otimiza_teste_explicar"));
    }

    #[test]
    fn o_mapa_do_disco_roda_nesta_maquina() {
        // MEDICAO DE VERDADE, na maquina que roda o teste. O mapa pode voltar
        // vazio (maquina sem nada), mas nao pode QUEBRAR -- e se voltar totais,
        // eles precisam ser coerentes.
        let mapa = mapear_o_disco(12).expect("o mapa do disco nao pode falhar");

        for pasta in &mapa.folders {
            assert!(pasta.percent >= 0.0 && pasta.percent <= 100.0,
                "{} tem porcentagem impossivel: {}", pasta.name, pasta.percent);
        }

        // O total nunca pode ser menor que a maior pasta.
        if let Some(maior) = mapa.folders.iter().map(|f| f.bytes).max() {
            assert!(mapa.total_bytes >= maior, "o total e menor que a maior pasta");
        }
    }
}
