// Otimizações para jogos em janela (Windows 11)
//
// O jogo em tela cheia sem borda passa pela composição da área de trabalho.
// Esta opção do Windows 11 troca o modelo antigo de apresentação pelo "flip",
// que tira esse atraso e libera VRR e Auto HDR no modo janela (Microsoft,
// "Optimizations for windowed games in Windows 11").
//
// O valor mora num texto só, junto de outras escolhas gráficas do Windows
// (`VRROptimizeEnable=0;AutoHDREnable=1;...`). Por isso a escrita mexe só no
// pedaço `SwapEffectUpgradeEnable` e preserva o resto: gravar o texto inteiro
// apagaria o Auto HDR ou o VRR que a pessoa escolheu.

use super::registry;
use crate::modules::changelog::ChangeRecord;

pub const CHAVE: &str = r"SOFTWARE\Microsoft\DirectX\UserGpuPreferences";
pub const VALOR: &str = "DirectXUserGlobalSettings";
const NOME: &str = "SwapEffectUpgradeEnable";

/// O primeiro build do Windows 11.
const WINDOWS_11: u32 = 22000;

/// O estado no texto: `Some(true)` ligado, `Some(false)` desligado, `None` sem
/// o pedaço (o padrão do Windows, que é desligado). **Função pura.**
pub fn ligada_no_texto(texto: &str) -> Option<bool> {
    texto.split(';').find_map(|par| {
        let (k, v) = par.split_once('=')?;
        (k.trim() == NOME).then(|| v.trim() == "1")
    })
}

/// O texto com o pedaço ligado, preservando o resto. **Função pura.**
pub fn com_ligada(texto: &str) -> String {
    let mut pares: Vec<String> = texto
        .split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty() && p.split_once('=').map(|(k, _)| k.trim()) != Some(NOME))
        .map(str::to_string)
        .collect();
    pares.push(format!("{}=1", NOME));
    format!("{};", pares.join(";"))
}

fn build_do_windows() -> u32 {
    registry::read_text("HKLM", r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "CurrentBuildNumber")
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0)
}

/// `Ok(None)`: a opção não existe neste Windows. `Err`: não deu para ler.
pub fn ligada() -> Result<Option<bool>, String> {
    if build_do_windows() < WINDOWS_11 {
        return Ok(None);
    }
    let texto = registry::read_text("HKCU", CHAVE, VALOR)?.unwrap_or_default();
    Ok(Some(ligada_no_texto(&texto) == Some(true)))
}

/// Liga a opção. Devolve a mudança para o histórico: desfazer devolve o texto
/// exato que havia antes.
pub fn ligar() -> Result<ChangeRecord, String> {
    // Leitura que falha não vira "não havia nada": o desfazer apagaria o que a
    // pessoa tinha (mesma regra de `gpupref.rs`).
    let atual = registry::read_text("HKCU", CHAVE, VALOR)?.unwrap_or_default();
    let anterior = registry::set_string("HKCU", CHAVE, VALOR, &com_ligada(&atual))?;
    Ok(ChangeRecord::RegistryValue {
        hive: "HKCU".to_string(),
        path: CHAVE.to_string(),
        name: VALOR.to_string(),
        previous: anterior,
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_o_estado_no_meio_do_texto() {
        assert_eq!(ligada_no_texto("VRROptimizeEnable=0;SwapEffectUpgradeEnable=1;"), Some(true));
        assert_eq!(ligada_no_texto("SwapEffectUpgradeEnable=0;AutoHDREnable=1;"), Some(false));
        assert_eq!(ligada_no_texto("AutoHDREnable=1;"), None);
        assert_eq!(ligada_no_texto(""), None);
    }

    #[test]
    fn ligar_preserva_as_outras_escolhas() {
        assert_eq!(com_ligada(""), "SwapEffectUpgradeEnable=1;");
        assert_eq!(com_ligada("AutoHDREnable=1;VRROptimizeEnable=0;"), "AutoHDREnable=1;VRROptimizeEnable=0;SwapEffectUpgradeEnable=1;");
        assert_eq!(com_ligada("SwapEffectUpgradeEnable=0;AutoHDREnable=1;"), "AutoHDREnable=1;SwapEffectUpgradeEnable=1;");
        assert_eq!(ligada_no_texto(&com_ligada("AutoHDREnable=1")), Some(true));
    }
}
