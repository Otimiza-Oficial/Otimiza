// MEMÓRIA PARA O JOGO. No PC do dono (8 GB), sem o FiveM aberto já sobrava só 1,4 GB: Brave 2,6 GB e Discord Canary
// 1,3 GB. Com o FiveM pedindo mais uns 3 GB, a memória acaba (medido: 0,24 GB livres, 3,1 GB indo para o disco) e o jogo
// trava quando precisa carregar cenário novo — o tranco no giro rápido de câmera. Nenhum ajuste do Windows resolve isso;
// liberar a memória resolve. Aqui: mostrar, medido, quem ocupa, e fechar só o que a pessoa escolher.
//
// Nada fecha sozinho. "Limpar RAM" (esvaziar memória dos processos) não entra: força mais leitura do disco depois.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Pesado {
    /// Nome do executável, sem `.exe` (o que a pessoa reconhece: brave, DiscordCanary).
    pub nome: String,
    pub mb: u64,
    pub processos: usize,
    /// O que se perde ao fechar (voz, música), quando há.
    pub aviso: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Memoria {
    pub total_gb: f64,
    pub livre_gb: f64,
    pub pesados: Vec<Pesado>,
}

/// Nunca oferecidos para fechar: o sistema, o antivírus, anticheats, o próprio Otimiza e o jogo.
const NUNCA: &[&str] = &[
    "system",
    "registry",
    "smss",
    "csrss",
    "wininit",
    "winlogon",
    "services",
    "lsass",
    "svchost",
    "dwm",
    "explorer",
    "fontdrvhost",
    "sihost",
    "ctfmon",
    "searchhost",
    "startmenuexperiencehost",
    "textinputhost",
    "runtimebroker",
    "msmpeng",
    "securityhealthservice",
    "memory compression",
    "secure system",
    "audiodg",
    "nvcontainer",
    "nvdisplay.container",
    "taskmgr",
    "pc-optimizer",
    "otimiza",
    // A tela do próprio Otimiza (e de outros programas) roda nele: fechar pelo nome derrubou o Otimiza no PC do dono
    // (09/10, 20:51, "WebView2: processo falhou").
    "msedgewebview2",
    "claude",
    "vgc",
    "vgtray",
    "easyanticheat",
    "beservice",
    "battleye",
    "faceit",
];

/// Prefixos que nunca entram: o jogo e o que ele abre (FiveM_ChromeBrowser, FiveM_GTAProcess…), e anticheats.
const PREFIXOS_NUNCA: &[&str] = &["fivem", "gta5", "gta_", "cfx", "eaanticheat", "anticheat"];

/// Pura: dos processos (nome, bytes), os que mais ocupam memória, agrupados por programa, a partir de 250 MB, os 6
/// maiores. `jogo`: o executável aberto, que nunca entra.
pub fn pesados(processos: &[(String, u64)], jogo: Option<&str>) -> Vec<Pesado> {
    use std::collections::HashMap;
    let jogo = jogo.map(|j| j.to_lowercase().trim_end_matches(".exe").to_string());
    let mut grupos: HashMap<String, (String, u64, usize)> = HashMap::new();
    for (nome, bytes) in processos {
        let exibido = nome
            .trim_end_matches(".exe")
            .trim_end_matches(".EXE")
            .to_string();
        let chave = exibido.to_lowercase();
        if NUNCA.contains(&chave.as_str())
            || PREFIXOS_NUNCA.iter().any(|p| chave.starts_with(p))
            || jogo.as_deref() == Some(chave.as_str())
        {
            continue;
        }
        let g = grupos.entry(chave).or_insert((exibido, 0, 0));
        g.1 += bytes;
        g.2 += 1;
    }
    let mut v: Vec<Pesado> = grupos
        .into_iter()
        .map(|(chave, (nome, bytes, n))| Pesado {
            aviso: aviso_de(&chave),
            nome,
            mb: bytes / 1_048_576,
            processos: n,
        })
        .filter(|p| p.mb >= 250)
        .collect();
    v.sort_by(|a, b| b.mb.cmp(&a.mb));
    v.truncate(6);
    v
}

fn aviso_de(chave: &str) -> Option<String> {
    if chave.starts_with("discord") || chave.contains("teamspeak") || chave == "ts3client" {
        Some("Se você usa a voz por ele no jogo, deixe aberto.".to_string())
    } else if chave.contains("spotify") {
        Some("A música para.".to_string())
    } else if ["brave", "chrome", "msedge", "firefox", "opera"]
        .iter()
        .any(|n| chave.starts_with(n))
    {
        Some("As abas voltam quando você abrir de novo.".to_string())
    } else {
        None
    }
}

/// Os pesados agora, medidos.
pub fn agora(jogo: Option<&str>) -> Memoria {
    use sysinfo::{ProcessesToUpdate, System};
    let mut s = System::new();
    s.refresh_memory();
    s.refresh_processes(ProcessesToUpdate::All, true);
    let lista: Vec<(String, u64)> = s
        .processes()
        .values()
        .map(|p| (p.name().to_string_lossy().to_string(), p.memory()))
        .collect();
    let gb = 1_073_741_824.0;
    Memoria {
        total_gb: s.total_memory() as f64 / gb,
        livre_gb: s.available_memory() as f64 / gb,
        pesados: pesados(&lista, jogo),
    }
}

/// Fecha o programa que a pessoa escolheu: primeiro pede para ele fechar (salva o que tem de salvar); se em 4 s ainda
/// estiver aberto (muitos só vão para a bandeja), encerra. Recusa o que está em `NUNCA` mesmo que peçam.
#[cfg(target_os = "windows")]
pub fn fechar(nome: &str) -> Result<(), String> {
    let chave = nome.to_lowercase().trim_end_matches(".exe").to_string();
    if chave.is_empty()
        || chave.contains(['\\', '/', '"', '*', '?'])
        || NUNCA.contains(&chave.as_str())
        || PREFIXOS_NUNCA.iter().any(|p| chave.starts_with(p))
    {
        return Err(format!("O Otimiza não fecha \"{}\".", nome));
    }
    let imagem = format!("{}.exe", nome.trim_end_matches(".exe"));
    let shell = crate::modules::windows::shell::run_com_prazo;
    let prazo = std::time::Duration::from_secs(10);
    let _ = shell("taskkill", &["/IM", &imagem, "/T"], prazo);
    std::thread::sleep(std::time::Duration::from_secs(4));
    let ainda = agora(None)
        .pesados
        .iter()
        .any(|p| p.nome.eq_ignore_ascii_case(nome))
        || processo_aberto(&chave);
    if ainda {
        let _ = shell("taskkill", &["/IM", &imagem, "/T", "/F"], prazo);
    }
    crate::utils::Logger::info(&format!("memória para o jogo: fechado {} a pedido", imagem));
    Ok(())
}

#[cfg(target_os = "windows")]
fn processo_aberto(chave: &str) -> bool {
    use sysinfo::{ProcessesToUpdate, System};
    let mut s = System::new();
    s.refresh_processes(ProcessesToUpdate::All, true);
    s.processes().values().any(|p| {
        p.name()
            .to_string_lossy()
            .to_lowercase()
            .trim_end_matches(".exe")
            == chave
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mb(n: u64) -> u64 {
        n * 1_048_576
    }

    #[test]
    fn agrupa_e_ordena_os_que_mais_ocupam() {
        // O PC do dono, medido: Brave em 16 processos, Discord Canary em 6.
        let mut l: Vec<(String, u64)> = (0..16)
            .map(|_| ("brave.exe".to_string(), mb(161)))
            .collect();
        l.extend((0..6).map(|_| ("DiscordCanary.exe".to_string(), mb(220))));
        l.push(("svchost.exe".to_string(), mb(537)));
        l.push(("FiveM_b3258_GTAProcess.exe".to_string(), mb(3100)));
        l.push(("FiveM_ChromeBrowser.exe".to_string(), mb(500)));
        l.push(("Lightshot.exe".to_string(), mb(24)));
        let p = pesados(&l, Some("FiveM_b3258_GTAProcess.exe"));
        assert_eq!(p.len(), 2, "{:?}", p);
        assert_eq!(p[0].nome, "brave");
        assert_eq!(p[0].mb, 2576);
        assert_eq!(p[0].processos, 16);
        assert!(p[0].aviso.as_deref().unwrap().contains("abas"));
        assert_eq!(p[1].nome, "DiscordCanary");
        assert!(p[1].aviso.as_deref().unwrap().contains("voz"));
    }

    #[test]
    fn nunca_oferece_sistema_jogo_nem_o_otimiza() {
        let l = vec![
            ("svchost.exe".to_string(), mb(900)),
            ("MsMpEng.exe".to_string(), mb(400)),
            ("FiveM.exe".to_string(), mb(800)),
            ("claude.exe".to_string(), mb(1200)),
            ("pc-optimizer.exe".to_string(), mb(300)),
            ("msedgewebview2.exe".to_string(), mb(600)),
        ];
        assert!(pesados(&l, None).is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn nunca_fecha_a_tela_do_proprio_otimiza() {
        assert!(fechar("msedgewebview2").is_err());
        assert!(fechar("msedgewebview2.exe").is_err());
    }
}
