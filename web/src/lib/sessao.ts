"use client";

import { useEffect, useState, useSyncExternalStore } from "react";
import { useRouter } from "next/navigation";
import { conferir, type Dados } from "./licenca";
import { ler, gravar, assinarArmazem, useLicencas, useCodigoDaMaquina } from "./armazem";

/*
 * A sessão, sem senha: a licença (assinatura Ed25519 de um computador) é o login, conferida no navegador.
 * Fica no localStorage deste navegador; sair é apagar. Não há sessão no servidor porque não há servidor.
 */

export type PassoOnboarding = "maquina" | "instalar" | "ativar" | "suporte";
export const PASSOS: PassoOnboarding[] = ["maquina", "instalar", "ativar", "suporte"];

export type Onboarding = {
  estado: "nao_iniciado" | "em_andamento" | "concluido";
  passo: number;
  feitos: PassoOnboarding[];
};

const VAZIO: Onboarding = { estado: "nao_iniciado", passo: 0, feitos: [] };
const CHAVE_ONBOARDING = "otimiza.painel.onboarding";

/*
 * Memorizado: objeto novo a cada renderização vira efeito em laço em quem depende dele.
 */
let ultimoOnboarding: string | null | undefined;
let ultimoValor: Onboarding = VAZIO;

function lerOnboarding(bruto: string | null): Onboarding {
  if (bruto === ultimoOnboarding) return ultimoValor;
  ultimoOnboarding = bruto;
  ultimoValor = interpretarOnboarding(bruto);
  return ultimoValor;
}

function interpretarOnboarding(bruto: string | null): Onboarding {
  if (!bruto) return VAZIO;
  try {
    const o = JSON.parse(bruto);
    const feitos = Array.isArray(o?.feitos) ? o.feitos.filter((f: unknown) => PASSOS.includes(f as PassoOnboarding)) : [];
    const estado = ["nao_iniciado", "em_andamento", "concluido"].includes(o?.estado) ? o.estado : "nao_iniciado";
    const passo = Number.isInteger(o?.passo) ? Math.min(Math.max(o.passo, 0), PASSOS.length - 1) : 0;
    return { estado, passo, feitos };
  } catch {
    return VAZIO;
  }
}

export function useOnboarding() {
  const bruto = useSyncExternalStore(assinarArmazem, () => ler(CHAVE_ONBOARDING), () => null);
  const onboarding = lerOnboarding(bruto);

  const salvar = (mudanca: Partial<Onboarding>) => {
    const atual = lerOnboarding(ler(CHAVE_ONBOARDING));
    gravar(CHAVE_ONBOARDING, JSON.stringify({ ...atual, ...mudanca }));
  };

  return {
    onboarding,
    comecar: () => salvar({ estado: "em_andamento" }),
    irPara: (passo: number) => salvar({ estado: "em_andamento", passo: Math.min(Math.max(passo, 0), PASSOS.length - 1) }),
    marcar: (passo: PassoOnboarding) => {
      const atual = lerOnboarding(ler(CHAVE_ONBOARDING));
      if (atual.feitos.includes(passo)) return;
      salvar({ feitos: [...atual.feitos, passo] });
    },
    concluir: () => salvar({ estado: "concluido" }),
    reiniciar: () => gravar(CHAVE_ONBOARDING, null),
  };
}

/*
 * Constantes de módulo: useSyncExternalStore reinscreve quando a função muda de identidade,
 * e a troca de rota (feita numa transição) nunca terminava.
 */
const SEM_INSCRICAO = () => () => {};
const ESTOU_NO_NAVEGADOR = () => true;
const ESTOU_NO_SERVIDOR = () => false;

export type Sessao = {
  carregando: boolean;
  entrou: boolean;
  onboarding: Onboarding;
  destino: "/entrar/" | "/boas-vindas/" | "/comecar/" | "/painel/";
};

/** "Entrou" é ter uma chave guardada, mesmo que não confira: a tela explica o motivo lá dentro. */
export function useSessao(): Sessao {
  const { licencas } = useLicencas();
  const { onboarding } = useOnboarding();
  // O servidor não tem localStorage: sem isto, a primeira pintura mandaria todo mundo ao login por um instante.
  const montou = useSyncExternalStore(SEM_INSCRICAO, ESTOU_NO_NAVEGADOR, ESTOU_NO_SERVIDOR);

  const entrou = licencas.length > 0;
  const destino = !entrou
    ? "/entrar/"
    : onboarding.estado === "concluido"
      ? "/painel/"
      : onboarding.estado === "em_andamento"
        ? "/comecar/"
        : "/boas-vindas/";

  return { carregando: !montou, entrou, onboarding, destino };
}

/** Só redireciona depois que o navegador respondeu, para não piscar a tela errada. */
export function useGuarda(permitidas: Sessao["destino"][]) {
  const sessao = useSessao();
  const router = useRouter();

  useEffect(() => {
    if (sessao.carregando) return;
    if (permitidas.includes(sessao.destino)) return;
    router.replace(sessao.destino);
  }, [sessao.carregando, sessao.destino, permitidas, router]);

  return sessao;
}

export function useLicencaAtiva() {
  const { licencas } = useLicencas();
  const [codigo] = useCodigoDaMaquina();
  // A resposta guarda qual pergunta respondeu: "conferindo" é calculado, e não escrito no efeito.
  const alvo = `${licencas.map((l) => l.chave).join("|")}::${codigo}`;
  const [resposta, setResposta] = useState<{ alvo: string; licenca: { chave: string; dados: Dados } | null } | null>(null);

  useEffect(() => {
    let vivo = true;
    (async () => {
      for (const l of licencas) {
        const r = await conferir(l.chave, null);
        if (!vivo) return;
        if (r.ok && (!codigo || r.dados.maquina === codigo)) {
          setResposta({ alvo, licenca: { chave: l.chave, dados: r.dados } });
          return;
        }
      }
      if (vivo) setResposta({ alvo, licenca: null });
    })();
    return () => {
      vivo = false;
    };
  }, [alvo, licencas, codigo]);

  const atual = resposta?.alvo === alvo ? resposta : null;
  return { licenca: atual?.licenca ?? null, conferindo: !atual, total: licencas.length };
}
