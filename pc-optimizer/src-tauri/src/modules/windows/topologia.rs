// Que núcleos este processador tem, segundo o Windows
//
// A CLASSE DE CADA NÚCLEO É INFORMAÇÃO DO SISTEMA, e não dedução pelo nome do
// processador. `GetLogicalProcessorInformationEx` devolve, para cada núcleo
// físico, a máscara dos lógicos dele e uma CLASSE DE EFICIÊNCIA — um número em
// que o maior é o mais rápido.
//
// Isso importa porque a alternativa seria uma tabela de "i9-14900HX tem 8
// núcleos P e 16 E", escrita à mão, que fica errada no lançamento seguinte e
// erra em todo processador que não estiver na lista. O Windows já sabe; basta
// perguntar.
//
// O NÚMERO NÃO É "P OU E", É UMA ESCALA
//
// `EfficiencyClass` é 0 para o mais lento e cresce. Num processador uniforme
// todos vêm 0 — e é por isso que "todos iguais" não é falha de leitura: é a
// resposta certa, e a maioria das máquinas responde assim. A conta é simples e
// está aqui: se existe mais de uma classe, os da MAIOR são os de desempenho.
//
// POR QUE A CHAMADA É FEITA DUAS VEZES
//
// A primeira só descobre o tamanho do buffer — é o padrão desta API do
// Windows, e ela DEVE falhar com `ERROR_INSUFFICIENT_BUFFER`. Qualquer outra
// resposta na primeira chamada é motivo para desistir, e não para chutar um
// tamanho.

#![cfg(target_os = "windows")]

use crate::modules::nucleos::{Classe, NucleoLogico, Topologia};

const RELATION_PROCESSOR_CORE: i32 = 0;
const ERROR_INSUFFICIENT_BUFFER: u32 = 122;

/// Lê a topologia desta máquina.
///
/// `None` quando o Windows não respondeu. Quem chama cai para uma topologia
/// uniforme montada com a contagem de lógicos — que é pior, mas é honesta:
/// sem a classe, o produto não oferece prender em núcleo nenhum.
pub fn ler() -> Option<Topologia> {
    use windows_sys::Win32::System::SystemInformation::{
        GetLogicalProcessorInformationEx, SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
    };

    let mut tamanho: u32 = 0;

    // A primeira chamada DEVE falhar pedindo buffer. Qualquer outra coisa é
    // desistir — chutar tamanho aqui daria leitura de memória fora do lugar.
    let primeira = unsafe {
        GetLogicalProcessorInformationEx(
            RELATION_PROCESSOR_CORE,
            std::ptr::null_mut(),
            &mut tamanho,
        )
    };
    if primeira != 0 || tamanho == 0 {
        return None;
    }
    if unsafe { windows_sys::Win32::Foundation::GetLastError() } != ERROR_INSUFFICIENT_BUFFER {
        return None;
    }

    let mut buffer = vec![0u8; tamanho as usize];
    let ok = unsafe {
        GetLogicalProcessorInformationEx(
            RELATION_PROCESSOR_CORE,
            buffer.as_mut_ptr() as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
            &mut tamanho,
        )
    };
    if ok == 0 {
        return None;
    }

    // Os registros têm TAMANHO VARIÁVEL e vêm colados: cada um diz o próprio
    // tamanho, e é por ele que se anda. Tratar o buffer como um vetor de
    // estruturas de tamanho fixo é o erro clássico desta API, e ele lê lixo a
    // partir do segundo núcleo.
    let mut cruas: Vec<(u64, u8)> = Vec::new();
    let mut deslocamento = 0usize;

    while deslocamento + std::mem::size_of::<u32>() * 2 <= tamanho as usize {
        let info = unsafe {
            &*(buffer.as_ptr().add(deslocamento) as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX)
        };

        let passo = info.Size as usize;
        if passo == 0 {
            break;
        }

        if info.Relationship == RELATION_PROCESSOR_CORE {
            let processador = unsafe { &info.Anonymous.Processor };

            // `GroupCount` maior que 1 significa processador com mais de 64
            // lógicos, dividido em grupos. A máscara simples não alcança isso,
            // e `nucleos::mascara_de` recusa índice acima de 63 — então aqui
            // só o primeiro grupo é lido, e a recusa acontece lá, com frase.
            let grupo = processador.GroupMask[0];
            cruas.push((grupo.Mask as u64, processador.EfficiencyClass));
        }

        deslocamento += passo;
    }

    if cruas.is_empty() {
        return None;
    }

    Some(montar(&cruas))
}

/// Transforma o que o Windows devolveu na topologia do produto.
///
/// Separada da chamada e pública para teste: a regra "a maior classe de
/// eficiência é a de desempenho" é decisão de produto, e ela precisa ser
/// testável sem ter um processador híbrido na mesa.
pub fn montar(cruas: &[(u64, u8)]) -> Topologia {
    let classes: Vec<u8> = {
        let mut c: Vec<u8> = cruas.iter().map(|(_, e)| *e).collect();
        c.sort_unstable();
        c.dedup();
        c
    };

    let hibrido = classes.len() > 1;
    let mais_rapida = classes.last().copied().unwrap_or(0);

    let mut nucleos = Vec::new();

    for (fisico, (mascara, eficiencia)) in cruas.iter().enumerate() {
        let classe = if !hibrido {
            Classe::Uniforme
        } else if *eficiencia == mais_rapida {
            Classe::Desempenho
        } else {
            Classe::Eficiencia
        };

        for indice in 0..64u32 {
            if mascara & (1u64 << indice) != 0 {
                nucleos.push(NucleoLogico {
                    indice,
                    fisico: fisico as u32,
                    classe,
                });
            }
        }
    }

    nucleos.sort_by_key(|n| n.indice);

    Topologia { nucleos, hibrido }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A regra de produto: com mais de uma classe, a MAIOR é a de desempenho.
    #[test]
    fn a_maior_classe_de_eficiencia_e_a_de_desempenho() {
        // Dois físicos rápidos com SMT (classe 1) e dois lentos (classe 0).
        let cruas = [(0b11u64, 1u8), (0b1100, 1), (0b10000, 0), (0b100000, 0)];

        let t = montar(&cruas);

        assert!(t.hibrido);
        assert_eq!(t.da_classe(Classe::Desempenho), vec![0, 1, 2, 3]);
        assert_eq!(t.da_classe(Classe::Eficiencia), vec![4, 5]);
    }

    /// Uma classe só é processador comum — e é a resposta certa, não falha.
    #[test]
    fn classe_unica_e_processador_uniforme() {
        let cruas = [(0b11u64, 0u8), (0b1100, 0)];

        let t = montar(&cruas);

        assert!(!t.hibrido);
        assert_eq!(t.da_classe(Classe::Uniforme).len(), 4);
        assert!(t.da_classe(Classe::Desempenho).is_empty());
    }

    #[test]
    fn os_logicos_do_mesmo_fisico_ficam_juntos() {
        let t = montar(&[(0b11u64, 0u8), (0b1100, 0)]);

        assert_eq!(t.quantos(), 4);
        assert_eq!(t.fisicos(), 2);
        assert_eq!(t.nucleos[0].fisico, t.nucleos[1].fisico);
        assert_ne!(t.nucleos[0].fisico, t.nucleos[2].fisico);
    }

    /// Os índices saem em ordem: a tela desenha a fileira de núcleos na ordem
    /// em que eles chegam, e fora de ordem a matriz fica embaralhada.
    #[test]
    fn os_indices_saem_em_ordem() {
        let t = montar(&[(0b110000u64, 1u8), (0b11, 0)]);

        let indices: Vec<u32> = t.nucleos.iter().map(|n| n.indice).collect();
        assert_eq!(indices, vec![0, 1, 4, 5]);
    }

    /// A leitura de verdade, nesta máquina.
    ///
    /// Não afirma quantos núcleos há — afirma o CONTRATO: o Windows responde,
    /// e o que ele responde bate com o que o sistema diz ter de processadores.
    #[test]
    fn a_topologia_desta_maquina_responde() {
        let Some(t) = ler() else {
            println!("o Windows não respondeu a topologia nesta máquina");
            return;
        };

        println!(
            "{} lógicos, {} físicos, híbrido: {}",
            t.quantos(),
            t.fisicos(),
            t.hibrido
        );
        if t.hibrido {
            println!(
                "  desempenho: {:?}",
                t.da_classe(crate::modules::nucleos::Classe::Desempenho)
            );
            println!(
                "  eficiência: {:?}",
                t.da_classe(crate::modules::nucleos::Classe::Eficiencia)
            );
        }

        assert!(t.quantos() > 0);
        assert!(t.fisicos() > 0);
        assert!(
            t.fisicos() <= t.quantos(),
            "físico não pode passar de lógico"
        );

        // Nenhum índice repetido: dois núcleos com o mesmo bit significaria
        // máscara errada, e é o defeito que prende o jogo no lugar errado.
        let mut indices: Vec<u32> = t.nucleos.iter().map(|n| n.indice).collect();
        let antes = indices.len();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices.len(), antes, "índice de núcleo repetido");
    }
}
