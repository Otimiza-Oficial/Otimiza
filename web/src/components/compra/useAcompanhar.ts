"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { consultar, FOLGA_DEPOIS_DO_PRAZO_MS, INTERVALO_CONSULTA_MS, type Compra, type Consulta, type Metodo } from "@/lib/api";

const CHAVE_SESSAO = "otimiza.compra";

export type CompraGuardada = { compra: Compra; metodo: Metodo; maquina: string };

/**
 * Só a compra em andamento, e só em `sessionStorage`: o token dá acesso a uma
 * licença e não deve sobreviver à aba num computador que pode ser de outro.
 */
export const sessao = {
  ler(): CompraGuardada | null {
    try {
      const valor = JSON.parse(sessionStorage.getItem(CHAVE_SESSAO) ?? "null");
      return valor?.compra?.token ? (valor as CompraGuardada) : null;
    } catch {
      return null;
    }
  },
  gravar(valor: CompraGuardada) {
    try {
      sessionStorage.setItem(CHAVE_SESSAO, JSON.stringify(valor));
    } catch {}
  },
  limpar() {
    try {
      sessionStorage.removeItem(CHAVE_SESSAO);
    } catch {}
  },
};

const FINAIS = new Set(["pago", "entregue", "recusada", "vencida", "cancelada", "estornada"]);

/**
 * Pergunta à API se a compra andou, até um estado final ou até o prazo da
 * própria compra passar. O prazo não recomeça ao recarregar a página.
 */
export function useAcompanhar() {
  const [restante, setRestante] = useState<number | null>(null);
  const parar = useRef<(() => void) | null>(null);

  const acompanhar = useCallback((compra: Compra, aoTerminar: (consulta: Consulta) => void) => {
    parar.current?.();
    const limite = compra.expiraEm + FOLGA_DEPOIS_DO_PRAZO_MS;

    const tique = () => setRestante(Math.max(0, Math.round((compra.expiraEm - Date.now()) / 1000)));
    tique();
    const relogio = setInterval(tique, 1000);

    const encerrar = (consulta: Consulta) => {
      clearInterval(relogio);
      clearInterval(consulta_);
      parar.current = null;
      setRestante(null);
      aoTerminar(consulta);
    };

    const consulta_ = setInterval(async () => {
      if (Date.now() > limite) return encerrar({ estado: "vencida" });
      try {
        const resposta = await consultar(compra.token);
        if (resposta && FINAIS.has(resposta.estado)) encerrar(resposta);
      } catch {
        // Rede oscilou: a próxima volta pergunta de novo.
      }
    }, INTERVALO_CONSULTA_MS);

    parar.current = () => {
      clearInterval(relogio);
      clearInterval(consulta_);
    };
  }, []);

  useEffect(() => () => parar.current?.(), []);

  return { acompanhar, restante, parar: () => parar.current?.() };
}

export function formatarRelogio(segundos: number) {
  const m = Math.floor(segundos / 60);
  const s = segundos % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}
