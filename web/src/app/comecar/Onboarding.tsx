"use client";

import {
  ArrowLeft,
  ArrowRight,
  ArrowUpRight,
  Check,
  Copy,
  Download,
  KeyRound,
  LifeBuoy,
  MonitorCheck,
  ShieldAlert,
} from "lucide-react";
import { useEffect, useId, useState, type ReactNode } from "react";
import { TelaDeEspera } from "@/app/boas-vindas/BoasVindas";
import { AuthLayout } from "@/components/auth/AuthLayout";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { Button, ButtonLink } from "@/components/ui/Button";
import { useCodigoDaMaquina, useLicencas } from "@/lib/armazem";
import { cn } from "@/lib/cn";
import { codigoValido, conferir } from "@/lib/licenca";
import { PASSOS, useGuarda, useLicencaAtiva, useOnboarding, type PassoOnboarding } from "@/lib/sessao";
import { links, site } from "@/lib/site";

const TITULOS: Record<PassoOnboarding, { rotulo: string; titulo: string; texto: string }> = {
  maquina: {
    rotulo: "Este computador",
    titulo: "Qual é o computador da sua licença?",
    texto:
      "O Otimiza mostra um código na tela de ativação, no formato OTZ-XXXX-XXXX-XXXX. Com ele, o painel sabe dizer se a chave que você colou é deste PC.",
  },
  instalar: {
    rotulo: "Instalação",
    titulo: "Baixe e instale o Otimiza.",
    texto:
      "São 5,8 MB. O diagnóstico completo roda antes de qualquer alteração — e o Windows vai avisar que o editor é desconhecido, porque o instalador ainda não tem assinatura digital.",
  },
  ativar: {
    rotulo: "Ativação",
    titulo: "Cole a sua chave no programa.",
    texto:
      "No Otimiza, o campo se chama “A sua chave”. A conferência acontece na sua máquina, sem consultar servidor nenhum — é a mesma que acabou de acontecer aqui.",
  },
  suporte: {
    rotulo: "Suporte",
    titulo: "Guarde onde pedir ajuda.",
    texto:
      "Trocou a placa-mãe, a chave parou de abrir, ficou uma dúvida: é no Discord, e quem responde é uma pessoa. Deixe o convite salvo antes de precisar.",
  },
};

export function Onboarding() {
  const sessao = useGuarda(["/comecar/"]);
  const { onboarding, irPara, marcar, concluir } = useOnboarding();
  const [concluindo, setConcluindo] = useState(false);

  if (sessao.carregando) return <TelaDeEspera />;

  const indice = Math.min(onboarding.passo, PASSOS.length - 1);
  const passo = PASSOS[indice];
  const ultimo = indice === PASSOS.length - 1;
  const { rotulo, titulo, texto } = TITULOS[passo];

  const seguir = () => {
    marcar(passo);
    if (!ultimo) {
      irPara(indice + 1);
      return;
    }
    // A guarda leva ao painel assim que o estado vira "concluído".
    setConcluindo(true);
    concluir();
  };

  return (
    <AuthLayout visual={<Previa passo={passo} />}>
      <Progresso indice={indice} total={PASSOS.length} feitos={onboarding.feitos.length} />

      <p className="eyebrow mt-8">{rotulo}</p>
      <h1 className="font-display mt-3 text-[28px] leading-[1.1] font-semibold tracking-[-0.045em]">{titulo}</h1>
      <p className="mt-3 text-[13.5px] leading-[1.6] text-muted">{texto}</p>

      <div className="mt-7">
        {passo === "maquina" && <PassoMaquina />}
        {passo === "instalar" && <PassoInstalar />}
        {passo === "ativar" && <PassoAtivar />}
        {passo === "suporte" && <PassoSuporte />}
      </div>

      <div className="mt-8 flex items-center gap-2">
        <Button
          variant="secondary"
          size="md"
          onClick={() => irPara(indice - 1)}
          disabled={indice === 0}
          aria-label="Voltar para o passo anterior"
        >
          <ArrowLeft size={15} strokeWidth={2} aria-hidden="true" />
        </Button>

        <Button size="md" className="flex-1" onClick={seguir} disabled={concluindo}>
          {ultimo ? "Ir para o painel" : "Continuar"}
          <ArrowRight size={15} strokeWidth={2} aria-hidden="true" />
        </Button>
      </div>

      <button
        type="button"
        onClick={concluir}
        className="mt-4 inline-flex min-h-8 w-full items-center justify-center text-[12.5px] text-muted transition-colors hover:text-fg"
      >
        Configurar depois
      </button>
    </AuthLayout>
  );
}

/* ------------------------------------------------------------------ progresso */

function Progresso({ indice, total, feitos }: { indice: number; total: number; feitos: number }) {
  return (
    <div>
      <div className="flex items-baseline justify-between">
        <p className="text-[12px] font-medium">
          Passo {indice + 1} <span className="text-subtle">de {total}</span>
        </p>
        {feitos > 0 && (
          <p className="flex items-center gap-1.5 text-[11.5px] text-muted">
            <Check size={12} strokeWidth={2.5} aria-hidden="true" />
            Salvo neste navegador
          </p>
        )}
      </div>
      <div className="mt-2 flex gap-1" role="progressbar" aria-valuenow={indice + 1} aria-valuemin={1} aria-valuemax={total}>
        {Array.from({ length: total }, (_, i) => (
          <span key={i} className={cn("h-[3px] flex-1 rounded-full", i <= indice ? "bg-ink" : "bg-[#e6e6e4]")} />
        ))}
      </div>
    </div>
  );
}

/* --------------------------------------------------------------------- passos */

function PassoMaquina() {
  const [codigo, setCodigo] = useCodigoDaMaquina();
  const { licencas } = useLicencas();
  const id = useId();
  const valido = codigoValido(codigo);

  // Com o código preenchido, dá para responder a pergunta que importa: a chave
  // guardada foi emitida para ESTA máquina? A resposta guarda para qual código
  // ela vale, para o texto nunca descrever um código anterior.
  const [resposta, setResposta] = useState<{ codigo: string; desta: boolean } | null>(null);

  useEffect(() => {
    if (!valido || licencas.length === 0) return;
    let vivo = true;
    (async () => {
      for (const l of licencas) {
        const r = await conferir(l.chave, codigo);
        if (!vivo) return;
        if (r.ok) {
          setResposta({ codigo, desta: true });
          return;
        }
      }
      if (vivo) setResposta({ codigo, desta: false });
    })();
    return () => {
      vivo = false;
    };
  }, [codigo, valido, licencas]);

  const confere = !valido || licencas.length === 0 ? null : resposta?.codigo === codigo ? (resposta.desta ? "desta" : "outra") : "conferindo";

  return (
    <div>
      <label htmlFor={id} className="text-[12.5px] font-semibold">
        Código deste computador
      </label>
      <input
        id={id}
        value={codigo}
        onChange={(e) => setCodigo(e.target.value)}
        placeholder="OTZ-XXXX-XXXX-XXXX"
        autoComplete="off"
        spellCheck={false}
        aria-invalid={codigo.length > 0 && !valido}
        className="campo mt-1.5 font-mono uppercase"
      />
      <p className="mt-2 text-[12px] leading-[1.5] text-muted">
        {codigo.length > 0 && !valido
          ? "Isso não tem a forma de um código do Otimiza: OTZ- e três blocos de quatro."
          : confere === "desta"
            ? "A sua chave foi emitida para este computador."
            : confere === "outra"
              ? "A chave guardada aqui é de outro computador. Se você trocou de PC, a reemissão é gratuita."
              : "Ainda não instalou? Pode deixar em branco e voltar aqui depois."}
      </p>
    </div>
  );
}

function PassoInstalar() {
  return (
    <div className="space-y-3">
      <ButtonLink href={links.baixar} size="md" className="w-full">
        <Download size={15} strokeWidth={2.25} aria-hidden="true" />
        Baixar o Otimiza {site.versao}
      </ButtonLink>

      <div className="rounded-[9px] bg-[#f6f6f5] p-3.5">
        <p className="flex items-center gap-2 text-[12.5px] font-semibold">
          <ShieldAlert size={14} strokeWidth={2} aria-hidden="true" />
          “O Windows protegeu o seu PC”
        </p>
        <p className="mt-1.5 text-[12px] leading-[1.55] text-muted">
          Clique em <span className="font-medium text-fg">Mais informações</span> e depois em{" "}
          <span className="font-medium text-fg">Executar assim mesmo</span>. O aviso aparece porque o instalador ainda
          não tem assinatura digital — está escrito nas notas de cada versão.
        </p>
      </div>
    </div>
  );
}

function PassoAtivar() {
  const { licencas } = useLicencas();
  const [copiada, setCopiada] = useState(false);
  const chave = licencas[0]?.chave ?? "";

  const copiar = async () => {
    try {
      await navigator.clipboard.writeText(chave);
      setCopiada(true);
      setTimeout(() => setCopiada(false), 1800);
    } catch {}
  };

  return (
    <div className="space-y-3">
      <ol className="space-y-2.5">
        {[
          "Abra o Otimiza. Ele mostra o código da máquina e o que achou no seu PC.",
          "Cole a chave no campo “A sua chave”.",
          "Clique em Ativar. Pronto — é para sempre, neste computador.",
        ].map((t, i) => (
          <li key={t} className="flex gap-3 text-[13px] leading-[1.5]">
            <span className="grid size-5 shrink-0 place-items-center rounded-full bg-[#f1f1f0] font-mono text-[10.5px] font-semibold">
              {i + 1}
            </span>
            {t}
          </li>
        ))}
      </ol>

      <Button variant="secondary" size="md" className="w-full" onClick={copiar} disabled={!chave}>
        {copiada ? <Check size={15} strokeWidth={2.5} aria-hidden="true" /> : <Copy size={15} strokeWidth={2} aria-hidden="true" />}
        {copiada ? "Chave copiada" : "Copiar a minha chave"}
      </Button>
    </div>
  );
}

function PassoSuporte() {
  return (
    <div className="space-y-3">
      <ButtonLink href={links.discord} size="md" className="w-full">
        <BrandLogo brand="discord" size={16} decorative />
        Entrar no Discord
        <ArrowUpRight size={15} strokeWidth={2} aria-hidden="true" />
      </ButtonLink>
      <p className="text-[12px] leading-[1.55] text-muted">
        É o mesmo lugar da compra. Formatar o Windows não custa chave nova; trocar a placa-mãe custa uma reemissão, que
        é gratuita.
      </p>
    </div>
  );
}

/* --------------------------------------------------------------------- prévia */

/** O que cada passo mostra do outro lado: a tela do programa naquele momento. */
function Previa({ passo }: { passo: PassoOnboarding }) {
  const { licenca } = useLicencaAtiva();
  const [codigo] = useCodigoDaMaquina();

  const conteudo: Record<PassoOnboarding, ReactNode> = {
    maquina: (
      <QuadroDoApp titulo="Ativar o Otimiza">
        <p className="text-[11px] font-medium text-muted">Código desta máquina</p>
        <p className="tabular mt-1 rounded-[7px] bg-[#f4f4f3] px-3 py-2 font-mono text-[12.5px]">
          {codigoValido(codigo) ? codigo : "OTZ-XXXX-XXXX-XXXX"}
        </p>
        <p className="mt-3 text-[11px] font-medium text-muted">A sua chave</p>
        <div className="mt-1 flex items-center gap-2 rounded-[7px] px-3 py-2 shadow-[0_0_0_1px_rgb(10_10_10/0.14)]">
          <KeyRound size={13} strokeWidth={2} className="shrink-0 text-subtle" aria-hidden="true" />
          <span className="truncate font-mono text-[12px] tracking-[0.12em]">●●●●●●●●●●●●</span>
        </div>
      </QuadroDoApp>
    ),
    instalar: (
      <QuadroDoApp titulo={`Otimiza-instalador.exe`}>
        <div className="flex items-center gap-3">
          <span className="grid size-10 place-items-center rounded-[9px] bg-ink text-white">
            <Download size={18} strokeWidth={2} aria-hidden="true" />
          </span>
          <div>
            <p className="text-[13px] font-semibold">Versão {site.versao}</p>
            <p className="text-[11.5px] text-muted">5,8 MB · Windows 10 e 11, 64 bits</p>
          </div>
        </div>
        <div className="mt-4 h-1.5 overflow-hidden rounded-full bg-[#ececeb]">
          <span className="block h-full w-2/3 rounded-full bg-ink" />
        </div>
      </QuadroDoApp>
    ),
    ativar: (
      <QuadroDoApp titulo="Licença ativada">
        <div className="flex items-center gap-3">
          <span className="grid size-9 place-items-center rounded-full bg-ink text-white">
            <Check size={16} strokeWidth={3} aria-hidden="true" />
          </span>
          <div className="min-w-0">
            <p className="text-[13px] font-semibold">Vitalícia, neste computador</p>
            <p className="truncate font-mono text-[11px] text-muted">
              {licenca?.dados.maquina ?? (codigo || "OTZ-XXXX-XXXX-XXXX")}
            </p>
          </div>
        </div>
      </QuadroDoApp>
    ),
    suporte: (
      <QuadroDoApp titulo="#suporte">
        <div className="flex items-center gap-3">
          <span className="grid size-9 place-items-center rounded-[9px] bg-[#f3f3f2]">
            <BrandLogo brand="discord" size={17} decorative />
          </span>
          <div>
            <p className="text-[13px] font-semibold">Otimiza · atendimento</p>
            <p className="text-[11.5px] text-muted">Quem emite a chave é uma pessoa.</p>
          </div>
        </div>
      </QuadroDoApp>
    ),
  };

  const rotulos: Record<PassoOnboarding, { Icone: typeof MonitorCheck; texto: string }> = {
    maquina: { Icone: MonitorCheck, texto: "É este código que amarra a chave ao seu PC." },
    instalar: { Icone: Download, texto: "O diagnóstico roda antes de qualquer alteração." },
    ativar: { Icone: KeyRound, texto: "A conferência acontece na sua máquina." },
    suporte: { Icone: LifeBuoy, texto: "O suporte é no mesmo lugar da compra." },
  };
  const { Icone, texto } = rotulos[passo];

  return (
    <div className="relative flex h-full flex-col justify-center px-10 py-12 xl:px-14">
      <div
        aria-hidden="true"
        className="absolute inset-0 [background-image:linear-gradient(rgba(10,10,10,.045)_1px,transparent_1px),linear-gradient(90deg,rgba(10,10,10,.045)_1px,transparent_1px)] [background-size:44px_44px] [mask-image:radial-gradient(ellipse_70%_60%_at_50%_45%,#000,transparent)]"
      />
      <div className="relative max-w-[420px]">
        <p className="flex items-center gap-2 text-[12px] font-medium text-muted">
          <Icone size={14} strokeWidth={2} aria-hidden="true" />
          {texto}
        </p>
        <div className="mt-4">{conteudo[passo]}</div>
      </div>
    </div>
  );
}

function QuadroDoApp({ titulo, children }: { titulo: string; children: ReactNode }) {
  return (
    <div className="rounded-[12px] bg-white p-4 shadow-[0_0_0_1px_rgb(10_10_10/0.08),0_16px_30px_-20px_rgb(10_10_10/0.4)]">
      <div className="flex items-center gap-2 border-b border-line pb-3">
        <OtimizaLogo mark={15} />
        <span className="text-[12px] font-semibold">{titulo}</span>
      </div>
      <div className="pt-3.5">{children}</div>
    </div>
  );
}
