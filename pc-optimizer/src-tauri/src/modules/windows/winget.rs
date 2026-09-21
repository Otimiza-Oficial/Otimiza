// O winget, que instala por nós — quando ele existe
//
// POR QUE O PRODUTO NÃO BAIXA NADA SOZINHO
//
// A aba de programas precisava de um jeito de instalar sem sair do Otimiza. As
// opções eram duas:
//
//   1. Baixar o instalador de um endereço nosso e rodar.
//   2. Pedir ao gerenciador de pacotes do próprio Windows.
//
// A primeira é o que um "otimizador" de vídeo do YouTube faz, e é
// indistinguível de entregar vírus: o cliente não tem como saber, antes de
// rodar, se aquele .exe é o 7-Zip ou outra coisa com o nome dele. A segunda põe
// a conta de "de onde veio este binário" na Microsoft, que mantém o
// repositório, verifica o fabricante e assina o catálogo.
//
// A ESCOLHA TEM UM PREÇO, E É ESTE MÓDULO: o winget pode não existir.
//
// NA MÁQUINA ONDE ISTO FOI ESCRITO, ELE NÃO EXISTE
//
// O pacote `Microsoft.DesktopAppInstaller` está instalado, na versão
// 1.0.30251.0, e `winget.exe` não está em lugar nenhum. É o estado que sobra
// de Windows "lite", debloat agressivo e Loja removida — exatamente o perfil do
// cliente que mais precisa de uma lista de programas básicos.
//
// Então a ausência não é caso de erro: é um dos dois estados normais deste
// módulo, e a tela precisa dizer qual deles vale.
//
// POR QUE NÃO SE PERGUNTA AO WINGET O QUE ESTÁ INSTALADO
//
// `winget list` demora segundos e às vezes quer rede. A lista de programas
// instalados sai das chaves de desinstalação do registro, que
// `conflicts::programas_instalados` já lê, que responde na hora, e que existe
// em toda máquina — inclusive nas que não têm winget.
//
// Uma pergunta, uma fonte: o registro responde "o que está instalado", o
// winget responde "instale isto". Nenhum dos dois responde a pergunta do outro.

#![cfg(target_os = "windows")]

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::shell;

/// Quanto tempo uma instalação pode levar antes de o produto desistir.
///
/// Dez minutos. Um runtime da Microsoft baixa em segundos; o Epic Games
/// Launcher numa conexão ruim passa de cinco. Desistir cedo demais deixaria
/// uma instalação pela metade, que é pior que esperar.
const PRAZO_DA_INSTALACAO: Duration = Duration::from_secs(600);

/// Em que pé está o winget nesta máquina.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum Disponibilidade {
    /// Achado e respondendo.
    Pronto { versao: String },
    /// Não existe nesta máquina.
    ///
    /// `como_resolver` é o que o técnico faz a respeito — e é a razão de este
    /// caso não ser um erro genérico.
    Ausente { como_resolver: String },
}

const COMO_RESOLVER: &str = "Esta máquina não tem o winget, o instalador de pacotes do Windows. \
     Ele vem com o \"Instalador de Aplicativo\" da Microsoft Store, e some quando a Loja é \
     removida num debloat. Instale o App Installer pela Loja, ou baixe os programas pelo site de \
     cada um — a lista abaixo continua dizendo o que já está instalado de qualquer jeito.";

/// Onde o winget costuma estar, quando o PATH não o entrega.
///
/// O executável vive dentro do pacote da Loja, e o atalho em `WindowsApps` é
/// o caminho público dele. Procurar direto aqui resolve o caso comum de o
/// processo ter herdado um PATH sem essa pasta.
fn caminhos_provaveis() -> Vec<PathBuf> {
    let mut caminhos = Vec::new();

    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        caminhos.push(
            PathBuf::from(local)
                .join("Microsoft")
                .join("WindowsApps")
                .join("winget.exe"),
        );
    }

    caminhos
}

/// O caminho do winget, se houver um.
pub fn onde_esta() -> Option<PathBuf> {
    caminhos_provaveis().into_iter().find(|c| c.is_file())
}

/// Pergunta ao winget a versão dele.
///
/// É a confirmação de que ele não só existe como RESPONDE. Um arquivo presente
/// e quebrado daria um botão que falha no clique, e o cliente leria isso como
/// defeito do Otimiza.
pub fn disponibilidade() -> Disponibilidade {
    let Some(caminho) = onde_esta() else {
        return Disponibilidade::Ausente {
            como_resolver: COMO_RESOLVER.to_string(),
        };
    };

    match shell::run(&caminho.to_string_lossy(), &["--version"]) {
        Ok(saida) if saida.success && !saida.stdout.trim().is_empty() => Disponibilidade::Pronto {
            versao: saida.stdout.trim().to_string(),
        },
        _ => Disponibilidade::Ausente {
            como_resolver: COMO_RESOLVER.to_string(),
        },
    }
}

/// Os argumentos da instalação.
///
/// Função pura e testada porque CADA UM DELES É UMA DECISÃO:
///
/// - `--id` com `--exact`: sem o exato, o winget faz busca por texto e pode
///   instalar um pacote de nome parecido. É o mesmo estrago do identificador
///   errado, por outro caminho.
/// - `--accept-package-agreements` e `--accept-source-agreements`: sem eles o
///   winget para esperando um "sim" que ninguém vai digitar, porque não há
///   terminal na frente do cliente. Aceitar aqui é aceitar em nome de quem
///   clicou no botão — e é por isso que a tela diz, antes do clique, que o
///   programa vem do fabricante e sob a licença dele.
/// - `--silent`: o instalador do fabricante não pode abrir uma janela própria
///   atrás do Otimiza, que o cliente não vê e não responde.
pub fn argumentos_de_instalacao(pacote: &str) -> Vec<String> {
    vec![
        "install".to_string(),
        "--id".to_string(),
        pacote.to_string(),
        "--exact".to_string(),
        "--silent".to_string(),
        "--accept-package-agreements".to_string(),
        "--accept-source-agreements".to_string(),
    ]
}

/// Instala um pacote. Devolve a saída do winget.
///
/// NÃO ESCONDE A SAÍDA quando dá errado: o texto do winget diz se foi rede,
/// se o pacote exige administrador ou se o instalador do fabricante recusou, e
/// essas são três conversas diferentes com o cliente.
pub fn instalar(pacote: &str) -> Result<String, String> {
    let Some(caminho) = onde_esta() else {
        return Err(COMO_RESOLVER.to_string());
    };

    let args = argumentos_de_instalacao(pacote);
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let saida = shell::run_com_prazo(&caminho.to_string_lossy(), &refs, PRAZO_DA_INSTALACAO)?;

    if saida.success {
        Ok(saida.stdout.trim().to_string())
    } else {
        // O texto do winget diz se foi rede, se o pacote exige administrador
        // ou se o instalador do fabricante recusou — três conversas
        // diferentes com o cliente, e um "falhou" genérico apaga as três.
        Err(format!(
            "{}{}",
            saida.stderr.trim(),
            if saida.stderr.trim().is_empty() {
                saida.stdout.trim().to_string()
            } else {
                String::new()
            }
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_instalacao_e_sempre_exata() {
        let args = argumentos_de_instalacao("7zip.7zip");

        assert!(args.contains(&"--exact".to_string()), "{args:?}");
        assert!(args.contains(&"--id".to_string()));
        assert!(args.contains(&"7zip.7zip".to_string()));
    }

    /// Sem aceitar os termos, o winget para esperando uma resposta que ninguém
    /// vai digitar — não há terminal na frente do cliente.
    #[test]
    fn a_instalacao_nao_fica_esperando_resposta() {
        let args = argumentos_de_instalacao("Valve.Steam");

        assert!(args.contains(&"--accept-package-agreements".to_string()));
        assert!(args.contains(&"--accept-source-agreements".to_string()));
        assert!(args.contains(&"--silent".to_string()));
    }

    #[test]
    fn o_pacote_entra_inteiro_e_sem_ser_interpretado() {
        // Nome com ponto e com sinal são comuns e não podem virar outra coisa.
        let args = argumentos_de_instalacao("Microsoft.VCRedist.2015+.x64");

        assert!(
            args.contains(&"Microsoft.VCRedist.2015+.x64".to_string()),
            "{args:?}"
        );
    }

    /// A ausência é um dos dois estados normais, e ela EXPLICA.
    ///
    /// Máquina sem winget não pode receber uma mensagem de erro genérica: o
    /// técnico precisa saber que é a Loja que falta, e que a lista continua
    /// servindo para conferir o que já está instalado.
    #[test]
    fn a_ausencia_vem_com_o_que_fazer() {
        match disponibilidade() {
            Disponibilidade::Pronto { versao } => {
                println!("winget presente: {versao}");
                assert!(!versao.is_empty());
            }
            Disponibilidade::Ausente { como_resolver } => {
                println!("winget ausente nesta máquina");
                assert!(como_resolver.contains("App Installer"));
                assert!(!como_resolver.is_empty());
            }
        }
    }
}
