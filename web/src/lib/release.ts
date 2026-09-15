"use client";

import { useEffect, useState } from "react";
import { site } from "./site";

/*
 * A versão mais nova, lida da API pública do GitHub no navegador de quem abre o
 * painel. O site é estático: não há servidor para perguntar por ele.
 *
 * Só o instalador de nome fixo aparece. O produto decidiu não mostrar a página
 * de releases nem os outros arquivos (.msi, setup com versão): cada escolha que
 * a pessoa não sabe fazer é uma desistência possível (`site/src/data/produto.ts`).
 */

export type Release = {
  versao: string;
  publicadaEm: string;
  resumo: string;
  instalador: { tamanhoBytes: number; url: string } | null;
};

export type EstadoRelease =
  | { estado: "carregando" }
  | { estado: "ok"; release: Release }
  | { estado: "falhou"; versaoConhecida: string };

const API = "https://api.github.com/repos/Otimiza-Oficial/Otimiza/releases/latest";
const CACHE = "otimiza.painel.release";
const VALIDADE_MS = 30 * 60 * 1000;

type Bruta = {
  tag_name: string;
  published_at: string;
  body: string | null;
  assets: { name: string; size: number; browser_download_url: string }[];
};

/** O primeiro trecho das notas, antes do primeiro título — sem tabela nem markdown. */
function resumir(corpo: string | null): string {
  if (!corpo) return "";
  const antesDoTitulo = corpo.split(/\n#{1,6}\s/)[0] ?? "";
  return antesDoTitulo.replace(/[*_`>]/g, "").replace(/\s+/g, " ").trim();
}

function converter(b: Bruta): Release {
  const exe = b.assets.find((a) => a.name === "Otimiza-instalador.exe");
  return {
    versao: b.tag_name.replace(/^v/, ""),
    publicadaEm: b.published_at,
    resumo: resumir(b.body),
    instalador: exe ? { tamanhoBytes: exe.size, url: exe.browser_download_url } : null,
  };
}

export function useUltimaRelease(): EstadoRelease {
  const [estado, setEstado] = useState<EstadoRelease>({ estado: "carregando" });

  useEffect(() => {
    let vivo = true;

    // A API sem autenticação aceita 60 pedidos por hora por endereço. O cache
    // de meia hora na sessão evita gastar isso trocando de página no painel.
    try {
      const guardado = JSON.parse(sessionStorage.getItem(CACHE) ?? "null");
      if (guardado && Date.now() - guardado.em < VALIDADE_MS) {
        queueMicrotask(() => vivo && setEstado({ estado: "ok", release: guardado.release }));
        return () => {
          vivo = false;
        };
      }
    } catch {}

    fetch(API, { headers: { Accept: "application/vnd.github+json" } })
      .then((r) => (r.ok ? (r.json() as Promise<Bruta>) : Promise.reject(r.status)))
      .then((bruta) => {
        const release = converter(bruta);
        try {
          sessionStorage.setItem(CACHE, JSON.stringify({ em: Date.now(), release }));
        } catch {}
        if (vivo) setEstado({ estado: "ok", release });
      })
      .catch(() => vivo && setEstado({ estado: "falhou", versaoConhecida: site.versao }));

    return () => {
      vivo = false;
    };
  }, []);

  return estado;
}

export const formatarTamanho = (bytes: number) =>
  `${(bytes / 1024 / 1024).toLocaleString("pt-BR", { maximumFractionDigits: 1 })} MB`;

export const formatarDataHora = (iso: string) =>
  new Date(iso).toLocaleDateString("pt-BR", { day: "2-digit", month: "long", year: "numeric" });
