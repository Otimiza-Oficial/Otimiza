"use client";

import { useSyncExternalStore } from "react";

/*
 * O que o painel guarda, e onde: no localStorage DESTE navegador, e em mais
 * lugar nenhum. Não há conta nem servidor. A chave guardada é conferida de novo
 * toda vez que aparece na tela — o que está no armazenamento não é confiado.
 */

export type LicencaGuardada = { chave: string; adicionadaEm: string };

const CHAVE_LICENCAS = "otimiza.painel.licencas";
const CHAVE_CODIGO = "otimiza.painel.codigo";
const EVENTO = "otimiza:armazem";

function ler(chave: string): string | null {
  try {
    return window.localStorage.getItem(chave);
  } catch {
    return null;
  }
}

function gravar(chave: string, valor: string | null) {
  try {
    if (valor === null) window.localStorage.removeItem(chave);
    else window.localStorage.setItem(chave, valor);
  } catch {
    // Janela anônima ou armazenamento bloqueado: o painel segue sem lembrar.
  }
  window.dispatchEvent(new Event(EVENTO));
}

function assinar(avisar: () => void) {
  window.addEventListener(EVENTO, avisar);
  window.addEventListener("storage", avisar);
  return () => {
    window.removeEventListener(EVENTO, avisar);
    window.removeEventListener("storage", avisar);
  };
}

/* O snapshot é a string crua: comparar string é estável entre renderizações,
   e o parse acontece fora do useSyncExternalStore. */
const semNada = () => null;

function parseLicencas(bruto: string | null): LicencaGuardada[] {
  if (!bruto) return [];
  try {
    const lista = JSON.parse(bruto);
    return Array.isArray(lista)
      ? lista.filter((l): l is LicencaGuardada => typeof l?.chave === "string" && typeof l?.adicionadaEm === "string")
      : [];
  } catch {
    return [];
  }
}

export function useLicencas() {
  const bruto = useSyncExternalStore(assinar, () => ler(CHAVE_LICENCAS), semNada);
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
  const codigo = useSyncExternalStore(assinar, () => ler(CHAVE_CODIGO), semNada) ?? "";
  const definir = (valor: string) => gravar(CHAVE_CODIGO, valor.trim() ? valor.trim().toUpperCase() : null);
  return [codigo, definir] as const;
}
