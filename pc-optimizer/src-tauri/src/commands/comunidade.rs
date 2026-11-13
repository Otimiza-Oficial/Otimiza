// Comunidade e nuvem (Plano 1.1, Onda 5): participar ou não, o que ganhou em PCs iguais, e o backup do histórico. Nada
// aqui muda o PC (`LIVRES`); o servidor confere a licença em cada pedido. Ver `plataforma::comunidade`.

use crate::plataforma::comunidade as c;

#[derive(serde::Deserialize)]
struct Erro {
    erro: Option<String>,
}

async fn erro_de(r: reqwest::Response) -> String {
    let status = r.status().as_u16();
    let e = r.json::<Erro>().await.ok().and_then(|e| e.erro);
    c::frase_do_erro(status, e.as_deref())
}

/// Liga ou desliga a participação. Desligar pede ao servidor para apagar o que esta licença mandou. `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn comunidade_participar(ligado: bool) -> Result<bool, String> {
    let mut p = crate::modules::preferences::Preferences::load();
    p.comunidade = ligado;
    p.save()?;
    if !ligado {
        if let (Ok((base, chave, maquina)), Ok(cl)) = (c::credenciais(), c::cliente()) {
            let r = cl
                .post(format!("{base}/v1/comunidade/esquecer"))
                .json(&serde_json::json!({ "chave": chave, "maquina": maquina }))
                .send()
                .await
                .map_err(|_| "Desliguei aqui, mas não consegui pedir ao servidor para apagar. Tente de novo depois.".to_string())?;
            if !r.status().is_success() {
                return Err(erro_de(r).await);
            }
        }
    }
    Ok(ligado)
}

/// O que mais ganhou em PCs com o mesmo processador e placa, neste jogo. Público (não precisa participar). `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn comunidade_parecidos(jogo: String) -> Result<c::PcsParecidos, String> {
    let base = crate::modules::licenca_online::endereco().ok_or("Esta versão não fala com o servidor do Otimiza.")?;
    let perfil = crate::plataforma::hardware::ultimo_conhecido().ok_or("Ainda não li as peças deste PC.")?;
    #[derive(serde::Deserialize)]
    struct Resposta {
        receitas: Vec<c::ReceitaDaComunidade>,
        minimo: u32,
    }
    let url = reqwest::Url::parse_with_params(
        &format!("{}/v1/comunidade/receitas", base.trim_end_matches('/')),
        &[("cpu", perfil.processador.as_str()), ("gpu", perfil.placa.nome.as_str()), ("jogo", jogo.as_str())],
    )
    .map_err(|_| c::frase_do_erro(0, None))?;
    let r = c::cliente()?
        .get(url)
        .send()
        .await
        .map_err(|_| c::frase_do_erro(0, None))?;
    if !r.status().is_success() {
        return Err(erro_de(r).await);
    }
    let corpo: Resposta = r.json().await.map_err(|_| c::frase_do_erro(0, None))?;
    Ok(c::PcsParecidos {
        processador: perfil.processador,
        placa: perfil.placa.nome,
        jogo,
        receitas: corpo.receitas,
        minimo: corpo.minimo,
    })
}

/// Guarda na nuvem o histórico e as preferências desta licença. Devolve a frase do que foi. `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn nuvem_guardar() -> Result<String, String> {
    let (base, chave, maquina) = c::credenciais()?;
    let agora = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (pacote, fora) = c::montar_pacote(&c::ler_arquivos(&c::pasta_de_dados()), agora);
    if pacote.arquivos.is_empty() {
        return Err("Ainda não há histórico para guardar.".to_string());
    }
    let dados = serde_json::to_string(&pacote).map_err(|e| e.to_string())?;
    let r = c::cliente()?
        .post(format!("{base}/v1/nuvem/guardar"))
        .json(&serde_json::json!({ "chave": chave, "maquina": maquina, "dados": dados }))
        .send()
        .await
        .map_err(|_| c::frase_do_erro(0, None))?;
    if !r.status().is_success() {
        return Err(erro_de(r).await);
    }
    let n = pacote.arquivos.len();
    let mut frase = format!("Guardado na nuvem: {n} {} ({} KB).", if n == 1 { "arquivo" } else { "arquivos" }, dados.len() / 1024 + 1);
    if !fora.is_empty() {
        frase.push_str(&format!(" Ficou de fora: {}.", fora.join(", ")));
    }
    Ok(frase)
}

/// Traz da nuvem o histórico e as preferências; a versão de antes de cada arquivo fica ao lado (`.antes-da-nuvem`).
/// `LIVRES`: só arquivos do próprio Otimiza.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn nuvem_restaurar() -> Result<String, String> {
    let (base, chave, maquina) = c::credenciais()?;
    #[derive(serde::Deserialize)]
    struct Resposta {
        dados: String,
    }
    let r = c::cliente()?
        .post(format!("{base}/v1/nuvem/ler"))
        .json(&serde_json::json!({ "chave": chave, "maquina": maquina }))
        .send()
        .await
        .map_err(|_| c::frase_do_erro(0, None))?;
    if !r.status().is_success() {
        return Err(erro_de(r).await);
    }
    let corpo: Resposta = r.json().await.map_err(|_| c::frase_do_erro(0, None))?;
    let pacote: c::Pacote =
        serde_json::from_str(&corpo.dados).map_err(|_| "O backup da nuvem veio num formato que não conheço.".to_string())?;
    let arquivos = c::a_restaurar(&pacote);
    let n = c::gravar_arquivos(&c::pasta_de_dados(), &arquivos)?;
    Ok(format!(
        "Restaurado: {n} {}. Abra o Otimiza de novo para tudo aparecer.",
        if n == 1 { "arquivo" } else { "arquivos" }
    ))
}
