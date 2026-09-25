// Auto CPU Set: prende o jogo nos núcleos de desempenho só quando MEDIDO que rende. Alterna todos e desempenho,
// 10 s cada, três rodadas: alternar impede que a partida ficar pesada com o tempo vire conclusão. Fica só se FPS
// médio ou 1% piores melhorarem de verdade (`repeticoes::comparar`) e nenhum piorar; empate volta para todos.
// Afinidade morre com o processo: por isso o vigia reaplica quando o jogo abre de novo.

#![cfg(target_os = "windows")]

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{afinidade, frames, topologia};
use crate::modules::nucleos;
use crate::modules::repeticoes::{self, Diferenca};

pub const SEGUNDOS_POR_FASE: u64 = 10;
pub const RODADAS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Escolha {
    SoDesempenho,
    TodosOsNucleos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultadoCpuSet {
    pub executavel: String,
    pub quando: u64,
    pub escolha: Escolha,
    pub fps_todos: f64,
    pub fps_desempenho: f64,
    pub low_todos: f64,
    pub low_desempenho: f64,
    pub diferenca_fps: Diferenca,
    pub diferenca_low: Diferenca,
}

pub fn decidir(fps: &Diferenca, low: &Diferenca) -> Escolha {
    let melhora = |d: &Diferenca| matches!(d, Diferenca::Real { delta, .. } if *delta > 0.0);
    let piora = |d: &Diferenca| matches!(d, Diferenca::Real { delta, .. } if *delta < 0.0);
    if (melhora(fps) || melhora(low)) && !piora(fps) && !piora(low) {
        Escolha::SoDesempenho
    } else {
        Escolha::TodosOsNucleos
    }
}

fn arquivo() -> PathBuf {
    let base = std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
    base.join("pc-optimizer").join("cpuset.json")
}

pub fn ler() -> BTreeMap<String, ResultadoCpuSet> {
    std::fs::read_to_string(arquivo())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn gravar(todos: &BTreeMap<String, ResultadoCpuSet>) -> Result<(), String> {
    let caminho = arquivo();
    if let Some(p) = caminho.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let texto = serde_json::to_string_pretty(todos).map_err(|e| e.to_string())?;
    std::fs::write(&caminho, texto).map_err(|e| format!("não consegui guardar o resultado: {e}"))
}

pub fn esquecer(executavel: &str) -> Result<(), String> {
    let mut todos = ler();
    todos.remove(&executavel.to_lowercase());
    gravar(&todos)
}

pub fn testar(pid: u32, executavel: &str) -> Result<ResultadoCpuSet, String> {
    let t = topologia::ler().ok_or("o Windows não informou a lista de núcleos desta máquina.")?;
    let mascara = nucleos::mascara_de_desempenho(&t).ok_or(
        "este processador tem todos os núcleos iguais: não existe núcleo melhor para testar.",
    )?;
    let nome = executavel.to_lowercase();

    let mut fps = (Vec::new(), Vec::new());
    let mut low = (Vec::new(), Vec::new());

    let medir = |prender: bool| -> Result<frames::FrameMeasurement, String> {
        if prender {
            afinidade::escrever(pid, mascara)?;
        } else {
            afinidade::soltar(pid)?;
        }
        // Um instante para o escalonador redistribuir as threads.
        std::thread::sleep(std::time::Duration::from_secs(1));
        frames::medir(pid, &nome, SEGUNDOS_POR_FASE)
    };

    for _ in 0..RODADAS {
        for prender in [false, true] {
            match medir(prender) {
                Ok(m) => {
                    let (f, l) = if prender { (&mut fps.1, &mut low.1) } else { (&mut fps.0, &mut low.0) };
                    f.push(m.fps);
                    l.push(m.low_1pct);
                }
                Err(e) => {
                    // Falhou no meio: o jogo volta para todos os núcleos.
                    let _ = afinidade::soltar(pid);
                    return Err(e);
                }
            }
        }
    }

    let resumo = |id: &str, v: &[f64]| repeticoes::resumir(id, v).ok_or("medição sem quadros".to_string());
    let (ft, fd) = (resumo("fps", &fps.0)?, resumo("fps", &fps.1)?);
    let (lt, ld) = (resumo("low", &low.0)?, resumo("low", &low.1)?);
    let diferenca_fps = repeticoes::comparar(&ft, &fd);
    let diferenca_low = repeticoes::comparar(&lt, &ld);
    let escolha = decidir(&diferenca_fps, &diferenca_low);

    match escolha {
        Escolha::SoDesempenho => afinidade::escrever(pid, mascara)?,
        Escolha::TodosOsNucleos => afinidade::soltar(pid)?,
    }

    let resultado = ResultadoCpuSet {
        executavel: nome.clone(),
        quando: crate::modules::changelog::now_timestamp(),
        escolha,
        fps_todos: ft.media,
        fps_desempenho: fd.media,
        low_todos: lt.media,
        low_desempenho: ld.media,
        diferenca_fps,
        diferenca_low,
    };
    let mut todos = ler();
    todos.insert(nome, resultado.clone());
    gravar(&todos)?;
    Ok(resultado)
}

/// Recusa do anticheat e processo que fechou não são erro: só não aplica.
pub fn reaplicar(ja: &mut HashSet<u32>) -> Vec<String> {
    let guardados: Vec<String> = ler()
        .into_iter()
        .filter(|(_, r)| r.escolha == Escolha::SoDesempenho)
        .map(|(exe, _)| exe)
        .collect();
    if guardados.is_empty() {
        ja.clear();
        return Vec::new();
    }
    let Some(t) = topologia::ler() else { return Vec::new() };
    let Some(mascara) = nucleos::mascara_de_desempenho(&t) else { return Vec::new() };

    let mut sistema = sysinfo::System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let vivos: HashSet<u32> = sistema.processes().keys().map(|p| p.as_u32()).collect();
    ja.retain(|p| vivos.contains(p));

    let mut aplicados = Vec::new();
    for (pid, processo) in sistema.processes() {
        let pid = pid.as_u32();
        if ja.contains(&pid) {
            continue;
        }
        let nome = processo.name().to_string_lossy().to_lowercase();
        if guardados.iter().any(|g| *g == nome) {
            ja.insert(pid);
            if afinidade::escrever(pid, mascara).is_ok() {
                aplicados.push(nome);
            }
        }
    }
    aplicados
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real(delta: f64) -> Diferenca {
        Diferenca::Real { delta, pct: Some(delta), folga: 0.5 }
    }
    fn empate() -> Diferenca {
        Diferenca::Indistinguivel { delta: 0.3, sobreposicao: 1.0 }
    }

    #[test]
    fn so_fica_nos_nucleos_de_desempenho_com_ganho_medido_e_sem_perda() {
        assert_eq!(decidir(&real(5.0), &empate()), Escolha::SoDesempenho);
        assert_eq!(decidir(&empate(), &real(3.0)), Escolha::SoDesempenho);
        assert_eq!(decidir(&real(5.0), &real(2.0)), Escolha::SoDesempenho);
    }

    #[test]
    fn nunca_menos_fps() {
        assert_eq!(decidir(&real(5.0), &real(-2.0)), Escolha::TodosOsNucleos);
        assert_eq!(decidir(&real(-4.0), &real(6.0)), Escolha::TodosOsNucleos);
        assert_eq!(decidir(&empate(), &empate()), Escolha::TodosOsNucleos);
        let sem = Diferenca::SemRepeticoes { falta: "x".into() };
        assert_eq!(decidir(&sem, &sem), Escolha::TodosOsNucleos);
    }
}
