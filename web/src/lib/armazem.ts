"use client";

import { useSyncExternalStore } from "react";

/*
 * O que o painel guarda, só no localStorage deste navegador. A chave guardada é conferida de novo toda vez que aparece.
 */

export type LicencaGuardada = { chave: string; adicionadaEm: string };

const CHAVE_LICENCAS = "otimiza.painel.licencas";
const CHAVE_CODIGO = "otimiza.painel.codigo";
const EVENTO = "otimiza:armazem";

export function ler(chave: string): string | null {
  try {
    return window.localStorage.getItem(chave);
  } catch {
    return null;
  }
}

export function gravar(chave: string, valor: string | null) {
  try {
    if (valor === null) window.localStorage.removeItem(chave);
    else window.localStorage.setItem(chave, valor);
  } catch {
    // Janela anônima ou armazenamento bloqueado: segue sem lembrar.
  }
  window.dispatchEvent(new Event(EVENTO));
}

export function assinarArmazem(avisar: () => void) {
  window.addEventListener(EVENTO, avisar);
  window.addEventListener("storage", avisar);
  return () => {
    window.removeEventListener(EVENTO, avisar);
    window.removeEventListener("storage", avisar);
  };
}

const semNada = () => null;

/*
 * Memorizado: um array novo a cada renderização disparava efeito em laço, e a troca de rota (numa transição) nunca terminava.
 */
let ultimoBruto: string | null | undefined;
let ultimaLista: LicencaGuardada[] = [];

function parseLicencas(bruto: string | null): LicencaGuardada[] {
  if (bruto === ultimoBruto) return ultimaLista;
  ultimoBruto = bruto;
  ultimaLista = (() => {
    if (!bruto) return [];
    try {
      const lista = JSON.parse(bruto);
      return Array.isArray(lista)
        ? lista.filter((l): l is LicencaGuardada => typeof l?.chave === "string" && typeof l?.adicionadaEm === "string")
        : [];
    } catch {
      return [];
    }
  })();
  return ultimaLista;
}

export function useLicencas() {
  const bruto = useSyncExternalStore(assinarArmazem, () => ler(CHAVE_LICENCAS), semNada);
  const licencas = parseLicencas(bruto);

  const adicionar = (chave: string) => {
    const limpa = chave.replace(/\s+/g, "");
    const atuais = parseLicencas(ler(CHAVE_LICENCAS));
    if (atuais.some((l) => l.chave === limpa)) return false;
    gravar(CHAVE_LICENCAS, JSON.stringify([{ chave: limpa, adicionadaEm: new Date().toISOString() }, ...atuais]));
    return true;
  };

  const remover = (chave: string) => {
    const restantes = parseLicencas(ler(CHAVE_LICENCAS)).filter((l) => l.chave !== chave);
    gravar(CHAVE_LICENCAS, restantes.length ? JSON.stringify(restantes) : null);
  };

  return { licencas, adicionar, remover };
}

export function useCodigoDaMaquina() {
  const codigo = useSyncExternalStore(assinarArmazem, () => ler(CHAVE_CODIGO), semNada) ?? "";
  const definir = (valor: string) => gravar(CHAVE_CODIGO, valor.trim() ? valor.trim().toUpperCase() : null);
  return [codigo, definir] as const;
}

/** O que "Sair" faz: não há sessão em servidor para encerrar. */
export function limparTudo() {
  for (const chave of [CHAVE_LICENCAS, CHAVE_CODIGO, "otimiza.painel.onboarding"]) gravar(chave, null);
}
