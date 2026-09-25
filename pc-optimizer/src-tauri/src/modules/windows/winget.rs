// Instalar pelo winget, e não baixar de endereço nosso: um .exe de servidor próprio é indistinguível de vírus
// para o cliente. O preço é que o winget pode não existir (Windows "lite", Loja removida), e isso é estado
// normal, com explicação. O que está instalado sai do registro, não do `winget list` (lento e às vezes com rede).

#![cfg(target_os = "windows")]

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::shell;

/// Dez minutos: desistir cedo deixaria instalação pela metade, que é pior que esperar.
const PRAZO_DA_INSTALACAO: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum Disponibilidade {
    Pronto { versao: String },
    /// `como_resolver` é o que o técnico faz a respeito.
    Ausente { como_resolver: String },
}

const COMO_RESOLVER: &str = "Esta máquina não tem o winget, o instalador de pacotes do Windows. \
     Ele vem com o \"Instalador de Aplicativo\" da Microsoft Store, e some quando a Loja é \
     removida num debloat. Instale o App Installer pela Loja, ou baixe os programas pelo site de \
     cada um — a lista abaixo continua dizendo o que já está instalado de qualquer jeito.";

/// O atalho público em `WindowsApps` resolve o processo que herdou um PATH sem essa pasta.
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

pub fn onde_esta() -> Option<PathBuf> {
    caminhos_provaveis().into_iter().find(|c| c.is_file())
}

/// Confirma que RESPONDE: arquivo presente e quebrado daria um botão que falha no clique.
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

/// `--exact`: sem ele o winget busca por texto e pode instalar pacote de nome parecido. Os dois `accept`: sem
/// eles o winget espera um "sim" que ninguém digita (a tela avisa antes que vem sob a licença do fabricante).
/// `--silent`: o instalador não pode abrir janela atrás do Otimiza.
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

/// Não esconde a saída: rede, administrador ou instalador que recusou são três conversas diferentes.
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

    #[test]
    fn a_instalacao_nao_fica_esperando_resposta() {
        let args = argumentos_de_instalacao("Valve.Steam");

        assert!(args.contains(&"--accept-package-agreements".to_string()));
        assert!(args.contains(&"--accept-source-agreements".to_string()));
        assert!(args.contains(&"--silent".to_string()));
    }

    #[test]
    fn o_pacote_entra_inteiro_e_sem_ser_interpretado() {
        let args = argumentos_de_instalacao("Microsoft.VCRedist.2015+.x64");

        assert!(
            args.contains(&"Microsoft.VCRedist.2015+.x64".to_string()),
            "{args:?}"
        );
    }

    /// Sem winget, a mensagem explica que falta a Loja, e que a lista continua servindo para conferir.
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
