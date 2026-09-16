"use client";

import { useEffect, useState, useSyncExternalStore } from "react";
import { useRouter } from "next/navigation";
import { conferir, type Dados } from "./licenca";
import { ler, gravar, assinarArmazem, useLicencas, useCodigoDaMaquina } from "./armazem";

/*
 * A SESSÃO DO OTIMIZA, E POR QUE ELA NÃO TEM SENHA.
 *
 * Não existe conta: a licença é uma assinatura Ed25519 emitida para um
 * computador, e é ela que prova quem é o cliente. Colar a chave AQUI é o
 * login — a conferência acontece no navegador, com a mesma chave pública do
 * programa, e nada é enviado para lugar nenhum.
 *
 * O que fica guardado é o que o cliente colou, no localStorage deste
 * navegador. Sair é apagar. Não há sessão no servidor porque não há servidor.
 */

export type PassoOnboarding = "maquina" | "instalar" | "ativar" | "suporte";
export const PASSOS: PassoOnboarding[] = ["maquina", "instalar", "ativar", "suporte"];

export type Onboarding = {
  estado: "nao_iniciado" | "em_andamento" | "concluido";
  /** Índice do passo em que a pessoa parou, para retomar onde largou. */
  passo: number;
  /** Passos que a pessoa marcou como feitos. */
  feitos: PassoOnboarding[];
};

const VAZIO: Onboarding = { estado: "nao_iniciado", passo: 0, feitos: [] };
const CHAVE_ONBOARDING = "otimiza.painel.onboarding";

/* Mesma memorização da lista de licenças, e pelo mesmo motivo: objeto novo a
   cada renderização vira efeito em laço em quem depender dele. */
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
 * Estas três são CONSTANTES DE MÓDULO, e não funções escritas na chamada.
 *
 * `useSyncExternalStore` reinscreve toda vez que a função de inscrição muda de
 * identidade — e escrita dentro do componente ela é nova a cada renderização.
 * O efeito colateral não é lentidão: é que a troca de rota, que o React faz
 * dentro de uma transição, nunca terminava. Clicar num item da lateral não
 * levava a lugar nenhum, e nenhum erro aparecia no console.
 */
const SEM_INSCRICAO = () => () => {};
const ESTOU_NO_NAVEGADOR = () => true;
const ESTOU_NO_SERVIDOR = () => false;

export type Sessao = {
  /** `null` enquanto o navegador ainda não respondeu o que tem guardado. */
  carregando: boolean;
  entrou: boolean;
  onboarding: Onboarding;
  /** Para onde esta pessoa deveria estar olhando agora. */
  destino: "/entrar/" | "/boas-vindas/" | "/comecar/" | "/painel/";
};

/**
 * Quem é esta pessoa e para onde ela vai.
 *
 * "Entrou" é ter pelo menos uma chave guardada aqui — não importa se ela
 * confere: a conferência acontece na tela, e uma chave que parou de valer
 * precisa levar a pessoa para dentro, para ela poder ler o motivo.
 */
export function useSessao(): Sessao {
  const { licencas } = useLicencas();
  const { onboarding } = useOnboarding();
  // O servidor não tem localStorage: sem este passo, a primeira pintura no
  // navegador decidiria o destino com a resposta do servidor (sempre vazia) e
  // mandaria todo mundo para o login por um instante.
  //
  // É `useSyncExternalStore` e não um `useEffect` que escreve estado: o React
  // já sabe responder "estou no servidor?" por aqui, sem uma renderização a
  // mais só para anotar isso.
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

/**
 * Manda a pessoa para onde ela deveria estar.
 *
 * `permitidas` são as rotas em que esta tela aceita ficar. O redirecionamento
 * só acontece depois que o navegador respondeu, para não piscar a tela errada.
 */
export function useGuarda(permitidas: Sessao["destino"][]) {
  const sessao = useSessao();
  const router = useRouter();

  useEffect(() => {
    if (sessao.carregando) return;
    if (permitidas.includes(sessao.destino)) return;
    router.replace(sessao.destino);
    // `permitidas` é literal em cada tela; comparar por conteúdo evita um
    // efeito que roda a cada renderização.
  }, [sessao.carregando, sessao.destino, permitidas, router]);

  return sessao;
}

/** A licença que vale para esta máquina, se houver alguma. */
export function useLicencaAtiva() {
  const { licencas } = useLicencas();
  const [codigo] = useCodigoDaMaquina();
  // A resposta guarda QUAL pergunta ela respondeu. Assim "conferindo" é
  // calculado — e não escrito dentro do efeito, que dispararia uma renderização
  // a mais a cada mudança de chave.
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
