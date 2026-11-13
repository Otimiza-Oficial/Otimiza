// Tela: o Hz que o monitor aguenta e o que ele está usando (`plataforma::hz`). A troca é em dois tempos, como o
// Windows faz: aplica sem gravar, a tela pergunta "Manter?" e, sem resposta em 15 s, volta sozinha.

use super::*;
use crate::plataforma::hz::{self, DiagnosticoDaTela, FimDaTroca, TrocaAplicada};

/// Um diagnóstico por monitor ativo: Hz atual, o máximo na resolução, a nativa, o cabo e o veredito. `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn tela_diagnostico() -> Vec<DiagnosticoDaTela> {
    hz::diagnosticar_telas()
}

/// Há troca esperando "Manter?" (a janela pode ter sido recarregada no meio). `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn tela_troca_pendente() -> bool {
    hz::troca_pendente()
}

/// Aplica SÓ a taxa, sem gravar, e arma a volta automática. A tela chama `tela_confirmar_troca` ou
/// `tela_voltar_troca` com o `token`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn tela_trocar_hz(
    dispositivo: String,
    hz: u32,
    state: State<'_, AppState>,
) -> Result<TrocaAplicada, String> {
    crate::modules::licenca::exigir()?;
    let log = state.changes.lock().await;
    hz::aplicar_hz(&dispositivo, hz, &log).map_err(|r| r.texto())
}

/// A pessoa viu a imagem: grava no Windows e no histórico. `LIVRES`: termina uma troca que já passou pela licença
/// em `tela_trocar_hz`; recusar aqui só deixaria a tela voltar sozinha.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn tela_confirmar_troca(token: u64, state: State<'_, AppState>) -> Result<FimDaTroca, String> {
    let mut log = state.changes.lock().await;
    hz::confirmar_troca(token, &mut log).map_err(|r| r.texto())
}

/// Volta antes do prazo. `LIVRES`: é desfazer.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn tela_voltar_troca(token: u64) -> Result<FimDaTroca, String> {
    hz::voltar_troca(token).map_err(|r| r.texto())
}
