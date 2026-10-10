// AS IMAGENS DO TOPO DO INÍCIO, escolhidas pela pessoa. Ficam só neste PC (`%APPDATA%\pc-optimizer\fundos`): o
// Otimiza não traz arte de jogo nenhuma no instalador — arte oficial é de quem fez o jogo. Sem imagem escolhida, o
// topo usa a arte da Steam deste PC (`capas::capa_larga`).

use base64::Engine;
use std::path::PathBuf;

/// Mais que isto é foto de câmera, não fundo, e pesa no IPC.
pub const TAMANHO_MAXIMO: usize = 6 * 1024 * 1024;
pub const QUANTAS: usize = 8;

fn pasta() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("fundos")
}

/// **Pura.** O tipo pela assinatura dos bytes, não pelo nome: só imagem entra.
pub fn tipo_da_imagem(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("jpg", "image/jpeg"))
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some(("png", "image/png"))
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(("webp", "image/webp"))
    } else {
        None
    }
}

fn arquivos() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(pasta())
        .map(|it| {
            it.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v.truncate(QUANTAS);
    v
}

/// As imagens como `data:` para o `<img>`/CSS. O que não for imagem (ou for grande demais) fica de fora.
pub fn listar() -> Vec<String> {
    arquivos()
        .into_iter()
        .filter_map(|p| {
            let bytes = std::fs::read(&p).ok()?;
            if bytes.len() > TAMANHO_MAXIMO {
                return None;
            }
            let (_, mime) = tipo_da_imagem(&bytes)?;
            Some(format!(
                "data:{};base64,{}",
                mime,
                base64::engine::general_purpose::STANDARD.encode(bytes)
            ))
        })
        .collect()
}

/// Recebe `data:image/...;base64,...` (o que o `FileReader` da tela entrega).
pub fn adicionar(data_url: &str) -> Result<usize, String> {
    let b64 = data_url
        .split_once(",")
        .map(|(_, b)| b)
        .ok_or("A imagem não chegou inteira.")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| "A imagem não chegou inteira.".to_string())?;
    if bytes.len() > TAMANHO_MAXIMO {
        return Err("Imagem grande demais: use uma de até 6 MB.".to_string());
    }
    let (ext, _) = tipo_da_imagem(&bytes).ok_or("Use uma imagem JPG, PNG ou WebP.")?;
    if arquivos().len() >= QUANTAS {
        return Err(format!(
            "Já são {} imagens. Remova as de agora antes de pôr outra.",
            QUANTAS
        ));
    }
    std::fs::create_dir_all(pasta()).map_err(|e| e.to_string())?;
    let agora = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let nome = format!("{}.{}", agora, ext);
    std::fs::write(pasta().join(nome), bytes)
        .map_err(|e| format!("Não consegui guardar a imagem: {}", e))?;
    Ok(arquivos().len())
}

/// Volta para a arte da Steam.
pub fn limpar() -> Result<(), String> {
    for p in arquivos() {
        std::fs::remove_file(&p)
            .map_err(|e| format!("Não consegui apagar {}: {}", p.display(), e))?;
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn so_imagem_entra() {
        assert_eq!(
            tipo_da_imagem(&[0xFF, 0xD8, 0xFF, 0xE0]).map(|t| t.0),
            Some("jpg")
        );
        assert_eq!(tipo_da_imagem(b"\x89PNG\r\n").map(|t| t.0), Some("png"));
        assert_eq!(
            tipo_da_imagem(b"RIFF\x00\x00\x00\x00WEBPVP8 ").map(|t| t.0),
            Some("webp")
        );
        assert_eq!(
            tipo_da_imagem(b"MZ\x90\x00"),
            None,
            "um .exe renomeado não entra"
        );
    }
}
