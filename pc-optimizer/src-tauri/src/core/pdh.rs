// Contadores de desempenho do Windows (PDH), num lugar só
//
// Os contadores são adicionados pelo NOME EM INGLÊS (`PdhAddEnglishCounterW`),
// que é o mesmo em qualquer idioma do Windows. Um contador que não existe
// nesta máquina (Windows modificado sem o serviço de contadores, placa sem
// driver de GPU com contador) fica com alça zero e devolve `None` — nunca 0.
//
// Documentação: learn.microsoft.com/windows/win32/perfctrs (PdhOpenQuery,
// PdhAddEnglishCounter, PdhCollectQueryData, PdhGetFormattedCounterValue,
// PdhGetFormattedCounterArray).

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhGetFormattedCounterValue, PdhOpenQueryW, PDH_FMT_COUNTERVALUE, PDH_FMT_COUNTERVALUE_ITEM_W,
        PDH_FMT_DOUBLE, PDH_MORE_DATA,
    };

    const PDH_OK: u32 = 0;
    /// Pede ao PDH que não limite o valor a 100 (a soma de motores de GPU e
    /// bytes de memória passam de 100 legitimamente).
    const PDH_FMT_NOCAP100: u32 = 0x0000_8000;

    fn largo(texto: &str) -> Vec<u16> {
        texto.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Alça de um contador dentro de uma consulta. Zero = não existe aqui.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Contador(isize);

    pub struct Consulta {
        alca: isize,
    }

    // A consulta PDH é usada por uma thread de cada vez (quem a possui).
    unsafe impl Send for Consulta {}

    impl Consulta {
        pub fn nova() -> Option<Self> {
            let mut alca: isize = 0;
            (unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut alca) } == PDH_OK).then_some(Consulta { alca })
        }

        pub fn adicionar(&self, caminho: &str) -> Contador {
            let mut h: isize = 0;
            let c = largo(caminho);
            if unsafe { PdhAddEnglishCounterW(self.alca, c.as_ptr(), 0, &mut h) } == PDH_OK {
                Contador(h)
            } else {
                Contador(0)
            }
        }

        /// Coleta uma amostra. Contadores de taxa (%, por segundo) só têm
        /// valor a partir da SEGUNDA coleta.
        pub fn coletar(&self) -> bool {
            unsafe { PdhCollectQueryData(self.alca) == PDH_OK }
        }

        pub fn valor(&self, c: Contador) -> Option<f64> {
            if c.0 == 0 {
                return None;
            }
            let mut v: PDH_FMT_COUNTERVALUE = unsafe { std::mem::zeroed() };
            let r = unsafe {
                PdhGetFormattedCounterValue(c.0, PDH_FMT_DOUBLE | PDH_FMT_NOCAP100, std::ptr::null_mut(), &mut v)
            };
            (r == PDH_OK).then(|| unsafe { v.Anonymous.doubleValue }).filter(|x| x.is_finite())
        }

        /// Todas as instâncias de um contador com curinga, com o nome de cada.
        /// Lista vazia quando o contador não existe ou não tem instância.
        pub fn lista(&self, c: Contador) -> Vec<(String, f64)> {
            if c.0 == 0 {
                return Vec::new();
            }
            let formato = PDH_FMT_DOUBLE | PDH_FMT_NOCAP100;
            let mut tamanho: u32 = 0;
            let mut itens: u32 = 0;
            let r = unsafe {
                PdhGetFormattedCounterArrayW(c.0, formato, &mut tamanho, &mut itens, std::ptr::null_mut())
            };
            if r != PDH_MORE_DATA || tamanho == 0 {
                return Vec::new();
            }
            let item = std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            // Alinhado para a estrutura: um Vec<u64> garante 8 bytes.
            let mut buffer = vec![0u64; (tamanho as usize + item) / 8 + 1];
            let r = unsafe {
                PdhGetFormattedCounterArrayW(
                    c.0,
                    formato,
                    &mut tamanho,
                    &mut itens,
                    buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W,
                )
            };
            if r != PDH_OK {
                return Vec::new();
            }
            let base = buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W;
            (0..itens as usize)
                .filter_map(|i| unsafe {
                    let it = &*base.add(i);
                    if it.szName.is_null() || it.FmtValue.CStatus != 0 {
                        return None;
                    }
                    let mut n = 0;
                    while *it.szName.add(n) != 0 {
                        n += 1;
                    }
                    let nome = String::from_utf16_lossy(std::slice::from_raw_parts(it.szName, n));
                    let v = it.FmtValue.Anonymous.doubleValue;
                    v.is_finite().then_some((nome, v))
                })
                .collect()
        }
    }

    impl Drop for Consulta {
        fn drop(&mut self) {
            unsafe { PdhCloseQuery(self.alca) };
        }
    }
}

#[cfg(windows)]
pub use imp::{Consulta, Contador};
