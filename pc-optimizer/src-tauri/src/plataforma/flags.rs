// Interruptores da 3.3. Cada tela ou motor novo entra desligado até estar pronto, e a tela antiga continua servindo
// quem já usa: nada de trocar tudo de uma vez. Ficam num arquivo próprio para o `preferences.json` de quem atualiza não
// mudar de forma.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Flags {
    /// Barra superior nova, com Início, Jogos, Boost, Lab e Ajustes.
    pub novo_shell: bool,
    pub nova_inicio: bool,
    pub biblioteca: bool,
    pub lab: bool,
    /// Escolha automática do melhor gerador de quadros legítimo.
    pub frame_boost_auto: bool,
    pub overlay: bool,
    /// Escreve no perfil do driver NVIDIA um ajuste que NÃO está no cabeçalho público. Desligado sempre por padrão.
    pub smooth_motion_drs: bool,
    /// Vibração digital da NVIDIA pelo NVAPI (números fora do cabeçalho público). Provada numa GTX 1650 (driver 617.14): ligada,
    /// e se o driver não responder à leitura o Otimiza cai sozinho para a rampa de gama.
    pub vibracao_nvidia: bool,
}

impl Default for Flags {
    fn default() -> Self {
        Flags {
            novo_shell: true,
            nova_inicio: true,
            biblioteca: true,
            lab: true,
            frame_boost_auto: true,
            // Pronto na 3.3: o interruptor fica como trava de emergência; quem liga é a pessoa, em Ajustes.
            overlay: true,
            smooth_motion_drs: false,
            vibracao_nvidia: true,
        }
    }
}

impl Flags {
    fn path() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        base.join("pc-optimizer").join("flags.json")
    }

    /// Ausente ou corrompido devolve o padrão: um interruptor não pode impedir o programa de abrir.
    pub fn load() -> Self {
        Self::ler(&Self::path())
    }

    fn ler(caminho: &Path) -> Self {
        fs::read_to_string(caminho)
            .ok()
            .and_then(|bruto| serde_json::from_str(&bruto).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        Self::gravar(&Self::path(), self)
    }

    fn gravar(caminho: &Path, flags: &Flags) -> Result<(), String> {
        if let Some(pasta) = caminho.parent() {
            fs::create_dir_all(pasta).map_err(|e| format!("Falha ao criar a pasta: {}", e))?;
        }

        let json = serde_json::to_string_pretty(flags)
            .map_err(|e| format!("Falha ao gravar os interruptores: {}", e))?;

        let temporario = crate::modules::changelog::caminho_temporario(caminho);
        fs::write(&temporario, json)
            .map_err(|e| format!("Falha ao gravar os interruptores: {}", e))?;
        fs::rename(&temporario, caminho)
            .map_err(|e| format!("Falha ao gravar os interruptores: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pasta(nome: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("otimiza-flags-{}", nome));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn o_ajuste_fora_do_cabecalho_publico_comeca_desligado() {
        assert!(!Flags::default().smooth_motion_drs);
    }

    #[test]
    fn arquivo_corrompido_volta_ao_padrao() {
        let caminho = pasta("corrompido").join("flags.json");
        fs::write(&caminho, "{nao e json").unwrap();

        assert_eq!(Flags::ler(&caminho), Flags::default());
    }

    #[test]
    fn interruptor_novo_ausente_no_arquivo_vem_do_padrao() {
        let caminho = pasta("parcial").join("flags.json");
        fs::write(&caminho, r#"{"lab": false}"#).unwrap();

        let lidas = Flags::ler(&caminho);
        assert!(!lidas.lab);
        assert!(lidas.novo_shell);
    }

    #[test]
    fn gravar_e_ler_devolve_o_mesmo() {
        let caminho = pasta("ida-e-volta").join("flags.json");
        let flags = Flags {
            overlay: true,
            ..Flags::default()
        };

        Flags::gravar(&caminho, &flags).unwrap();
        assert_eq!(Flags::ler(&caminho), flags);
    }
}
