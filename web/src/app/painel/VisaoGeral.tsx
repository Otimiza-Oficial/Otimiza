"use client";

import { ArrowRight, Download, KeyRound, LifeBuoy, MonitorCheck, Sparkles } from "lucide-react";
import Link from "next/link";
import type { ReactNode } from "react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { Bloco, PainelTitulo } from "@/components/painel/PainelShell";
import { StatusLicenca, useConferencia } from "@/components/painel/Licenca";
import { ButtonLink } from "@/components/ui/Button";
import { useCodigoDaMaquina, useLicencas } from "@/lib/armazem";
import { codigoValido, formatarData } from "@/lib/licenca";
import { formatarDataHora, formatarTamanho, useUltimaRelease } from "@/lib/release";
import { links, PRECO_BRL, site } from "@/lib/site";

export function VisaoGeral() {
  const release = useUltimaRelease();
  const { licencas } = useLicencas();
  const [codigo] = useCodigoDaMaquina();

  const versao =
    release.estado === "ok" ? release.release.versao : release.estado === "falhou" ? release.versaoConhecida : null;

  return (
    <>
      <PainelTitulo
        titulo="Seu painel"
        texto="A sua licença, o instalador mais novo e o caminho até o suporte. Não há conta nem senha: o que você guarda aqui fica neste navegador."
      />

      <div className="mt-8 grid gap-4 sm:grid-cols-3">
        <Numero
          icone={<Sparkles size={16} strokeWidth={2} aria-hidden="true" />}
          rotulo="Versão mais nova"
          valor={versao ? versao : "…"}
          nota={
            release.estado === "ok"
              ? `Publicada em ${formatarDataHora(release.release.publicadaEm)}`
              : release.estado === "falhou"
                ? "Não deu para consultar o GitHub agora"
                : "Consultando o GitHub"
          }
        />
        <Numero
          icone={<KeyRound size={16} strokeWidth={2} aria-hidden="true" />}
          rotulo="Chaves neste navegador"
          valor={String(licencas.length)}
          nota={licencas.length ? "Conferidas de novo a cada visita" : "Nenhuma guardada ainda"}
        />
        <Numero
          icone={<MonitorCheck size={16} strokeWidth={2} aria-hidden="true" />}
          rotulo="Este computador"
          valor={codigo && codigoValido(codigo) ? codigo : "—"}
          mono={Boolean(codigo && codigoValido(codigo))}
          nota={codigo && codigoValido(codigo) ? "Código informado por você" : "Informe em Licenças"}
        />
      </div>

      <div className="mt-5 grid items-start gap-5 lg:grid-cols-[minmax(0,1.35fr)_minmax(0,1fr)]">
        <Bloco className="p-5 sm:p-6">
          <div className="flex items-center justify-between gap-3">
            <h2 className="font-display text-[18px] font-semibold tracking-[-0.03em]">Suas licenças</h2>
            <Link href="/painel/licencas/" className="flex min-h-10 items-center gap-1 text-[13px] font-medium text-muted hover:text-fg lg:min-h-0">
              Gerenciar <ArrowRight size={14} strokeWidth={2} aria-hidden="true" />
            </Link>
          </div>

          {licencas.length === 0 ? (
            <div className="mt-5">
              <p className="text-[14px] leading-[1.6] text-muted">
                Ainda não há nenhuma chave aqui. Se você já comprou, confira e guarde a sua. Se ainda não, o caminho é
                este:
              </p>
              <ol className="mt-5 space-y-3">
                {[
                  "Baixe e instale o Otimiza — o diagnóstico completo já roda antes de pagar.",
                  "Copie o código da máquina que aparece na tela (OTZ-XXXX-XXXX-XXXX).",
                  `No Discord, mande o código e pague R$ ${PRECO_BRL}, uma vez. A chave é emitida para esse código.`,
                ].map((passo, i) => (
                  <li key={passo} className="flex gap-3 text-[13.5px] leading-[1.55]">
                    <span className="grid size-6 shrink-0 place-items-center rounded-full bg-[#f1f1f0] font-mono text-[11px] font-semibold">
                      {i + 1}
                    </span>
                    {passo}
                  </li>
                ))}
              </ol>
              <div className="mt-6 flex flex-wrap gap-2">
                <ButtonLink href="/painel/licencas/">
                  <KeyRound size={14} strokeWidth={2} aria-hidden="true" />
                  Conferir minha chave
                </ButtonLink>
                <ButtonLink href={links.comprar} variant="secondary">
                  <BrandLogo brand="discord" size={15} decorative />
                  Comprar no Discord
                </ButtonLink>
              </div>
            </div>
          ) : (
            <ul className="mt-4 divide-y divide-line">
              {licencas.slice(0, 4).map((l) => (
                <LinhaLicenca key={l.chave} chave={l.chave} maquina={codigo && codigoValido(codigo) ? codigo : null} />
              ))}
            </ul>
          )}
        </Bloco>

        <div className="space-y-5">
          <Bloco escuro className="p-5 sm:p-6">
            <p className="eyebrow text-[#a3a3a3]">Instalador</p>
            <p className="font-display mt-3 text-[22px] font-semibold tracking-[-0.04em]">Otimiza {versao ?? site.versao}</p>
            <p className="mt-1.5 text-[13px] text-[#b5b5b5]">
              Windows 10 e 11, 64 bits
              {release.estado === "ok" && release.release.instalador
                ? ` · ${formatarTamanho(release.release.instalador.tamanhoBytes)}`
                : ""}
            </p>
            <ButtonLink href={links.baixar} variant="light" size="md" className="mt-6 w-full">
              <Download size={15} strokeWidth={2.25} aria-hidden="true" />
              Baixar o instalador
            </ButtonLink>
          </Bloco>

          <Bloco className="p-5 sm:p-6">
            <p className="flex items-center gap-2 text-[14px] font-semibold">
              <LifeBuoy size={16} strokeWidth={2} aria-hidden="true" />
              Precisa de ajuda?
            </p>
            <p className="mt-1.5 text-[13px] leading-[1.55] text-muted">
              Trocou a placa-mãe, a chave não abre, ou ficou alguma dúvida — tem mensagem pronta para mandar.
            </p>
            <Link
              href="/painel/suporte/"
              className="mt-2 inline-flex min-h-10 items-center gap-1 text-[13px] font-semibold hover:underline"
            >
              Abrir o suporte <ArrowRight size={14} strokeWidth={2} aria-hidden="true" />
            </Link>
          </Bloco>
        </div>
      </div>

      {release.estado === "ok" && release.release.resumo && (
        <Bloco className="mt-5 p-5 sm:p-6">
          <p className="eyebrow">Sobre a versão {release.release.versao}</p>
          <p className="mt-3 max-w-[760px] text-[14px] leading-[1.65] text-[#2b2b2b]">{release.release.resumo}</p>
        </Bloco>
      )}
    </>
  );
}

function Numero({
  icone,
  rotulo,
  valor,
  nota,
  mono = false,
}: {
  icone: ReactNode;
  rotulo: string;
  valor: string;
  nota: string;
  mono?: boolean;
}) {
  return (
    <Bloco className="p-5">
      <p className="flex items-center gap-2 text-[12.5px] font-medium text-muted">
        {icone}
        {rotulo}
      </p>
      <p
        className={
          mono
            ? "mt-3 truncate font-mono text-[18px] font-semibold tracking-[-0.01em]"
            : "font-display tabular mt-3 text-[30px] leading-none font-semibold tracking-[-0.05em]"
        }
      >
        {valor}
      </p>
      <p className="mt-2 text-[12px] text-subtle">{nota}</p>
    </Bloco>
  );
}

function LinhaLicenca({ chave, maquina }: { chave: string; maquina: string | null }) {
  const resultado = useConferencia(chave, maquina);
  const dados = resultado?.ok ? resultado.dados : null;
  return (
    <li className="flex items-center justify-between gap-4 py-3.5">
      <div className="min-w-0">
        <p className="truncate font-mono text-[13px] font-medium">{dados ? dados.maquina : `${chave.slice(0, 16)}…`}</p>
        <p className="mt-0.5 text-[12px] text-subtle">
          {dados
            ? `Emitida em ${formatarData(dados.emitida)}${dados.comprador ? ` · ${dados.comprador}` : ""}`
            : resultado
              ? "Não confere — veja em Licenças"
              : "Conferindo"}
        </p>
      </div>
      <StatusLicenca resultado={resultado} className="shrink-0" />
    </li>
  );
}
