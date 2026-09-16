"use client";

import { ArrowRight, Download, House, KeyRound, MonitorCheck, RefreshCw, Sparkles } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { Banner } from "@/components/painel/Banner";
import { CabecalhoPagina, Cartao } from "@/components/painel/AppShell";
import { CartaoMetrica, Selo } from "@/components/painel/CartaoMetrica";
import { Checklist } from "@/components/painel/Estados";
import { FiltroPeriodo } from "@/components/painel/FiltroPeriodo";
import { GraficoVersoes } from "@/components/painel/GraficoVersoes";
import { ListaMaquinas } from "@/components/painel/ListaMaquinas";
import { useCodigoDaMaquina, useLicencas } from "@/lib/armazem";
import { cn } from "@/lib/cn";
import { codigoValido, formatarData } from "@/lib/licenca";
import { formatarDataHora, useUltimaRelease } from "@/lib/release";
import { useLicencaAtiva, useOnboarding } from "@/lib/sessao";
import { links, site } from "@/lib/site";

/*
 * O PAINEL TEM DOIS ESTADOS, E ELES NÃO SE MISTURAM.
 *
 * Quem acabou de entrar não instalou nada: mostrar curva, histórico e lista
 * cheia para essa pessoa seria simular uma operação que não existe. Ela vê a
 * lista do que falta e os estados vazios.
 *
 * Quem já informou a máquina e tem a chave conferida vê o painel completo.
 * Nenhum número aqui é inventado: os que existem vêm da chave (assinada) e da
 * API pública do GitHub.
 */
export function VisaoGeral() {
  const { licenca, conferindo, total } = useLicencaAtiva();
  const [codigo] = useCodigoDaMaquina();
  const { licencas } = useLicencas();
  const { onboarding } = useOnboarding();
  const release = useUltimaRelease();
  const [meses, setMeses] = useState(12);
  const [atualizando, setAtualizando] = useState(false);

  const temMaquina = codigoValido(codigo);
  const configurado = temMaquina && Boolean(licenca);
  const versao = release.estado === "ok" ? release.release.versao : release.estado === "falhou" ? release.versaoConhecida : null;

  const atualizar = () => {
    setAtualizando(true);
    try {
      sessionStorage.removeItem("otimiza.painel.release");
      sessionStorage.removeItem("otimiza.painel.versoes");
    } catch {}
    setTimeout(() => window.location.reload(), 150);
  };

  return (
    <div className="space-y-3">
      <Banner />

      <CabecalhoPagina
        icone={<House size={14} strokeWidth={2} aria-hidden="true" />}
        titulo="Início"
        texto={
          configurado
            ? "O resumo da sua licença e do programa."
            : "Três passos separam a sua chave de um PC medido."
        }
        acoes={
          <>
            <button
              type="button"
              onClick={atualizar}
              className="flex h-[26px] items-center gap-1.5 rounded-[6px] bg-white px-2 text-[11.5px] font-medium text-[#3a3a3a] shadow-[0_0_0_1px_rgb(10_10_10/0.1)] transition-colors hover:bg-[#f9f9f8]"
            >
              <RefreshCw size={12} strokeWidth={2} aria-hidden="true" className={cn(atualizando && "animate-spin")} />
              Atualizar
            </button>
            <FiltroPeriodo valor={meses} aoEscolher={setMeses} />
          </>
        }
      />

      {/* quatro números, e todos vêm de algum lugar conferível */}
      <div className="grid grid-cols-2 gap-2 xl:grid-cols-4">
        <CartaoMetrica
          rotulo="Licença"
          valor={conferindo ? "…" : licenca ? (licenca.dados.expira ? "Válida" : "Vitalícia") : total ? "Não confere" : "Nenhuma"}
          nota={licenca ? `Emitida em ${formatarData(licenca.dados.emitida)}` : "Confira a sua chave em Licenças"}
          etiqueta={licenca && <Selo tom="escuro">Ativa</Selo>}
        />
        <CartaoMetrica
          rotulo="Computador"
          valor={temMaquina ? codigo : "—"}
          mono={temMaquina}
          nota={temMaquina ? "Código informado por você" : "Informe para conferir a chave"}
          etiqueta={<MonitorCheck size={13} strokeWidth={2} className="text-subtle" aria-hidden="true" />}
        />
        <CartaoMetrica
          rotulo="Versão mais nova"
          valor={versao ?? "…"}
          nota={
            release.estado === "ok"
              ? `Publicada em ${formatarDataHora(release.release.publicadaEm)}`
              : release.estado === "falhou"
                ? "Sem resposta do GitHub agora"
                : "Consultando o GitHub"
          }
          etiqueta={<Sparkles size={13} strokeWidth={2} className="text-subtle" aria-hidden="true" />}
        />
        <CartaoMetrica
          rotulo="Chaves aqui"
          valor={String(licencas.length)}
          nota={licencas.length ? "Guardadas só neste navegador" : "Nenhuma guardada ainda"}
          etiqueta={<KeyRound size={13} strokeWidth={2} className="text-subtle" aria-hidden="true" />}
        />
      </div>

      {!configurado && (
        <Checklist
          titulo="Comece por aqui"
          itens={[
            { id: "chave", titulo: "Chave conferida", detalhe: "Assinatura conferida neste navegador", feito: total > 0 },
            {
              id: "maquina",
              titulo: "Informar o código deste computador",
              detalhe: "Aparece na tela de ativação do Otimiza",
              feito: temMaquina,
              href: "/painel/licencas/",
            },
            {
              id: "instalar",
              titulo: "Baixar e instalar o Otimiza",
              detalhe: "Grátis, 5,8 MB, Windows 10 e 11",
              feito: onboarding.feitos.includes("instalar"),
              href: "/painel/downloads/",
            },
            {
              id: "suporte",
              titulo: "Entrar no Discord do suporte",
              detalhe: "É onde a chave é reemitida, sem custo",
              feito: onboarding.feitos.includes("suporte"),
              href: "/painel/suporte/",
            },
          ]}
        />
      )}

      {/* a área analítica: ritmo das versões + as máquinas desta conta */}
      <div className="grid gap-2 lg:grid-cols-[minmax(0,1.9fr)_minmax(0,1fr)]">
        <Cartao>
          <div className="flex items-center justify-between gap-3 border-b border-line px-3.5 py-2.5">
            <div>
              <h2 className="text-[12.5px] font-semibold">Ritmo das versões</h2>
              <p className="text-[11px] text-subtle">Quando cada versão do Otimiza saiu</p>
            </div>
            <Link
              href="/painel/downloads/"
              className="flex items-center gap-1 text-[11.5px] font-medium text-muted transition-colors hover:text-fg"
            >
              Ver downloads
              <ArrowRight size={12} strokeWidth={2} aria-hidden="true" />
            </Link>
          </div>
          <div className="px-2 pt-2 pb-1">
            <GraficoVersoes periodoEmMeses={meses} />
          </div>
          <p className="border-t border-line px-3.5 py-2 text-[11px] text-subtle">
            Dados públicos das versões do Otimiza. O programa não envia nada da sua máquina.
          </p>
        </Cartao>

        <Cartao className="overflow-hidden">
          <div className="flex items-center justify-between gap-3 border-b border-line px-3.5 py-2.5">
            <h2 className="text-[12.5px] font-semibold">Seus computadores</h2>
            <Link
              href="/painel/licencas/"
              className="flex items-center gap-1 text-[11.5px] font-medium text-muted transition-colors hover:text-fg"
            >
              Ver todas
              <ArrowRight size={12} strokeWidth={2} aria-hidden="true" />
            </Link>
          </div>
          <ListaMaquinas />
        </Cartao>
      </div>

      {/* rodapé de ações: o que a pessoa faz daqui */}
      <div className="grid gap-2 sm:grid-cols-3">
        <AcaoRapida
          href={links.baixar}
          externo
          icone={<Download size={14} strokeWidth={2} aria-hidden="true" />}
          titulo={`Baixar o Otimiza ${site.versao}`}
          texto="Instalador para Windows 10 e 11"
        />
        <AcaoRapida
          href="/painel/licencas/"
          icone={<KeyRound size={14} strokeWidth={2} aria-hidden="true" />}
          titulo="Conferir outra chave"
          texto="Para um segundo computador"
        />
        <AcaoRapida
          href={links.discord}
          externo
          icone={<BrandLogo brand="discord" size={14} decorative />}
          titulo="Falar no Discord"
          texto="Reemissão, dúvidas e compra"
        />
      </div>
    </div>
  );
}

function AcaoRapida({
  href,
  icone,
  titulo,
  texto,
  externo = false,
}: {
  href: string;
  icone: React.ReactNode;
  titulo: string;
  texto: string;
  externo?: boolean;
}) {
  const conteudo = (
    <>
      <span className="grid size-8 shrink-0 place-items-center rounded-[7px] bg-[#f4f4f3]">{icone}</span>
      <span className="min-w-0">
        <span className="block truncate text-[12.5px] font-semibold">{titulo}</span>
        <span className="block truncate text-[11.5px] text-subtle">{texto}</span>
      </span>
      <ArrowRight size={13} strokeWidth={2} className="ml-auto shrink-0 text-subtle" aria-hidden="true" />
    </>
  );
  const classe =
    "flex items-center gap-2.5 rounded-[8px] bg-white px-3 py-2.5 shadow-[0_0_0_1px_rgb(10_10_10/0.07),0_1px_2px_rgb(10_10_10/0.04)] transition-colors hover:bg-[#fbfbfa]";

  return externo ? (
    <a href={href} target="_blank" rel="noopener noreferrer" className={classe}>
      {conteudo}
    </a>
  ) : (
    <Link href={href} className={classe}>
      {conteudo}
    </Link>
  );
}
