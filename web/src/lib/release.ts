"use client";

import { useEffect, useState } from "react";
import { site } from "./site";

/*
 * A versão mais nova, lida da API pública do GitHub no navegador. Só o instalador de nome fixo aparece:
 * cada escolha que a pessoa não sabe fazer é uma desistência possível.
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

    // 60 pedidos por hora por endereço sem autenticação: o cache de meia hora na sessão poupa isso.
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

export type ItemDeVersao = { versao: string; publicadaEm: string };

export type EstadoHistorico =
  | { estado: "carregando" }
  | { estado: "ok"; versoes: ItemDeVersao[] }
  | { estado: "falhou" };

const CACHE_HISTORICO = "otimiza.painel.versoes";

/** Todas as versões publicadas: o único dado com linha do tempo que a área do cliente tem, já que o programa não manda nada. */
export function useHistoricoDeVersoes(): EstadoHistorico {
  const [estado, setEstado] = useState<EstadoHistorico>({ estado: "carregando" });

  useEffect(() => {
    let vivo = true;

    try {
      const guardado = JSON.parse(sessionStorage.getItem(CACHE_HISTORICO) ?? "null");
      if (guardado && Date.now() - guardado.em < VALIDADE_MS) {
        queueMicrotask(() => vivo && setEstado({ estado: "ok", versoes: guardado.versoes }));
        return () => {
          vivo = false;
        };
      }
    } catch {}

    fetch("https://api.github.com/repos/Otimiza-Oficial/Otimiza/releases?per_page=100", {
      headers: { Accept: "application/vnd.github+json" },
    })
      .then((r) => (r.ok ? (r.json() as Promise<Bruta[]>) : Promise.reject(r.status)))
      .then((brutas) => {
        const versoes = brutas
          .map((b) => ({ versao: b.tag_name.replace(/^v/, ""), publicadaEm: b.published_at }))
          .sort((a, b) => a.publicadaEm.localeCompare(b.publicadaEm));
        try {
          sessionStorage.setItem(CACHE_HISTORICO, JSON.stringify({ em: Date.now(), versoes }));
        } catch {}
        if (vivo) setEstado({ estado: "ok", versoes });
      })
      .catch(() => vivo && setEstado({ estado: "falhou" }));

    return () => {
      vivo = false;
    };
  }, []);

  return estado;
}
