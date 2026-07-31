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
    /// Verdadeiro quando a soma parou no limite de profundidade e o número
    /// mostrado é um piso, não o total.
    pub partial: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderMap {
    pub root: String,
    pub total_bytes: u64,
    pub total_formatted: String,
    pub folders: Vec<FolderEntry>,
    /// Quantas pastas não puderam ser lidas por falta de permissão.
    pub unreadable: usize,
    /// Verdadeiro quando a varredura parou no prazo. Os totais viram piso, não
    /// medida final — e a interface precisa dizer isso.
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
        v.ilegiveis += 1;
        return (0, false);
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
    let mut algum_cortado = false;
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

        total += bytes;
        ilegiveis += v.ilegiveis;
        algum_cortado |= partial;

        let name = caminho
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        pastas.push(FolderEntry {
            explanation: explicar(&name).to_string(),
            formatted: format_size(bytes),
            path: caminho.to_string_lossy().to_string(),
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
        timed_out: algum_cortado,
    })
}

/// A pasta do usuário: onde está o espaço que ele mesmo pode decidir sobre.
pub fn perfil_do_usuario() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Users"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn pasta_vazia_nao_gera_divisao_por_zero() {
        let temporaria = std::env::temp_dir().join("otimiza_teste_mapa_vazio");
        std::fs::create_dir_all(&temporaria).expect("criar pasta de teste");

        let mapa = mapear(&temporaria, 5).expect("pasta vazia é legível");

        assert_eq!(mapa.total_bytes, 0);
        assert!(mapa.folders.is_empty());

        let _ = std::fs::remove_dir(&temporaria);
    }
}
