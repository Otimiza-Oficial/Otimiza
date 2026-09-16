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
    // Janela anônima ou armazenamento bloqueado: o painel segue sem lembrar.
  }
  window.dispatchEvent(new Event(EVENTO));
}

/** Avisa quem estiver na tela quando qualquer coisa guardada aqui mudar. */
export function assinarArmazem(avisar: () => void) {
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

/*
 * O RESULTADO É MEMORIZADO, E ISSO NÃO É MICRO-OTIMIZAÇÃO.
 *
 * Sem isto, cada renderização devolvia um ARRAY NOVO para o mesmo texto
 * guardado. Todo efeito que depende da lista — conferir a chave, montar a busca,
 * listar as máquinas — via a dependência mudar, rodava de novo, gravava estado,
 * e disparava a renderização seguinte: um laço infinito de efeito.
 *
 * A tela continuava respondendo a clique, então o defeito não aparecia. O que
 * ele quebrava era a NAVEGAÇÃO: o React troca de rota dentro de uma transição,
 * e uma transição não termina enquanto a árvore não para de se atualizar.
 * Clicar em "Downloads" não levava a lugar nenhum, sem nenhum erro no console.
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

/**
 * Apaga tudo o que o painel guardou neste navegador: chaves, código da máquina
 * e o andamento do primeiro acesso. É o que "Sair" faz — não há sessão em
 * servidor nenhum para encerrar.
 */
export function limparTudo() {
  for (const chave of [CHAVE_LICENCAS, CHAVE_CODIGO, "otimiza.painel.onboarding"]) gravar(chave, null);
}
