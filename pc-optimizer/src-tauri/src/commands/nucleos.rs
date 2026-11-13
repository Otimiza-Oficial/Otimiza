// Processador: núcleos da máquina, prender o jogo em núcleos (cpuset) e a investigação das travadas.
// Ficam juntos porque o teste da travada e o cpuset mexem na mesma afinidade do processo do jogo.

use super::*;

/// SÓ LÊ. A classe de cada núcleo vem do Windows, não de tabela de modelos. Processador uniforme: NÃO HÁ O QUE
/// FAZER, escrito (um botão ali seria um jeito de piorar).
#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn nucleos_da_maquina() -> Result<NucleosNaTela, String> {
    use crate::modules::nucleos;
    use crate::modules::windows::{afinidade, deteccao, topologia};

    let (topologia, jogo) =
        tokio::task::spawn_blocking(|| (topologia::ler(), deteccao::procurar()))
            .await
            .map_err(|e| format!("a leitura dos núcleos não terminou: {e}"))?;

    let topologia = topologia.ok_or(
        "o Windows não informou a lista de núcleos desta máquina. Sem ela o produto não oferece \
         prender o jogo em núcleo nenhum — prender no núcleo errado é pior que não prender.",
    )?;

    // A afinidade atual diz se o jogo JÁ está preso, por exemplo nos núcleos de eficiência por outro programa.
    let (jogo_nome, jogo_pid, jogo_mascara, jogo_exe) = match &jogo {
        Some(j) => {
            let mascara = afinidade::ler(j.pid).ok().map(|(processo, _)| processo);
            (
                Some(j.nome.clone()),
                Some(j.pid),
                mascara,
                Some(j.executavel.clone()),
            )
        }
        None => (None, None, None, None),
    };

    Ok(NucleosNaTela {
        conselho: nucleos::conselho(&topologia),
        // Daqui, não da tela: duas versões da conta de físicos discordariam.
        fisicos: topologia.fisicos(),
        mascara_de_desempenho: nucleos::mascara_de_desempenho(&topologia).map(|m| m.to_string()),
        topologia,
        jogo_nome,
        jogo_pid,
        jogo_mascara: jogo_mascara.map(|m| m.to_string()),
        jogo_exe,
    })
}

#[cfg(target_os = "windows")]
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct NucleosNaTela {
    pub topologia: crate::modules::nucleos::Topologia,
    pub conselho: crate::modules::nucleos::Conselho,
    pub fisicos: usize,
    /// TEXTO: uma máscara de 64 bits perde os bits altos no número do JavaScript.
    pub mascara_de_desempenho: Option<String>,
    pub jogo_nome: Option<String>,
    pub jogo_pid: Option<u32>,
    pub jogo_mascara: Option<String>,
    pub jogo_exe: Option<String>,
}

/// NÃO entra no histórico: afinidade morre com o processo, e uma linha lá ofereceria para sempre desfazer um
/// processo que não existe. O desfazer é o botão ao lado, ou fechar o jogo.
#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn prender_jogo_nos_nucleos(pid: u32, prender: bool) -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    use crate::modules::nucleos;
    use crate::modules::windows::{afinidade, topologia};

    tokio::task::spawn_blocking(move || {
        if !prender {
            afinidade::soltar(pid)?;
            return Ok("O jogo voltou a poder usar todos os núcleos.".to_string());
        }

        let t = topologia::ler().ok_or("o Windows não informou a lista de núcleos.")?;

        // Processador uniforme não tem núcleo melhor: prender só tira máquina.
        let mascara = nucleos::mascara_de_desempenho(&t).ok_or(
            "este processador tem todos os núcleos iguais: prender o jogo em parte deles só \
             reduziria o que a máquina entrega.",
        )?;

        afinidade::escrever(pid, mascara)?;

        Ok(format!(
            "O jogo está preso nos {} núcleos de desempenho. Isso vale para o processo aberto e \
             some quando o jogo fechar.",
            nucleos::indices_de(mascara).len()
        ))
    })
    .await
    .map_err(|e| format!("a mudança não terminou: {e}"))?
}

/// Auto CPU Set (2.9): mede o jogo em todos os núcleos e só nos de desempenho, alternando, e fica com o que rendeu.
#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn cpuset_testar(
    pid: u32,
    executavel: String,
) -> Result<crate::modules::windows::cpuset::ResultadoCpuSet, String> {
    crate::modules::licenca::exigir()?;
    tokio::task::spawn_blocking(move || {
        // A vez da prova alternada: o teste do Turbo depois da prova também troca a afinidade do jogo.
        let _vez = crate::modules::provaalternada::reservar()
            .ok_or("Uma prova está rodando: espere ela terminar para testar os núcleos.")?;
        crate::modules::windows::cpuset::testar(pid, &executavel)
    })
    .await
    .map_err(|e| format!("o teste não terminou: {e}"))?
}

/// `LIVRES`: só mede (PresentMon e contadores do Windows); nada é alterado. Motor V2 da causa da travada.
#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn investigar_travadas(
    segundos: u32,
) -> Result<crate::modules::windows::investigacao::ResultadoDaInvestigacao, String> {
    let segundos = segundos.clamp(30, 120);
    tokio::task::spawn_blocking(move || crate::modules::windows::investigacao::investigar(segundos))
        .await
        .map_err(|e| format!("a investigação não terminou: {e}"))?
}

/// `EXIGEM_LICENCA`: acalma o programa suspeito em janelas alternadas e devolve; só fica acalmado com a causa
/// demonstrada, e até o jogo fechar.
#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn testar_causa_da_travada(
    processo: String,
) -> Result<crate::modules::windows::investigacao::ResultadoDoTeste, String> {
    crate::modules::licenca::exigir()?;
    let app = crate::aplicativo()?;
    tokio::task::spawn_blocking(move || {
        use tauri::Emitter;
        crate::modules::windows::investigacao::testar(&processo, &|frase| {
            let _ = app.emit("travada:progresso", frase);
        })
    })
    .await
    .map_err(|e| format!("o teste não terminou: {e}"))?
}

/// `LIVRES`: só desfaz o que o teste deixou acalmado.
#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn devolver_teste_da_travada() -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(crate::modules::windows::investigacao::devolver_mantido)
        .await
        .map_err(|e| format!("a devolução não terminou: {e}"))?
}

#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn cpuset_resultados(
) -> Result<Vec<crate::modules::windows::cpuset::ResultadoCpuSet>, String> {
    Ok(crate::modules::windows::cpuset::ler()
        .into_values()
        .collect())
}

/// `LIVRES`: só tira.
#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn cpuset_esquecer(executavel: String) -> Result<(), String> {
    crate::modules::windows::cpuset::esquecer(&executavel)
}
