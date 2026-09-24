"use client";

import { ArrowRight, Download, House, KeyRound, MonitorCheck, RefreshCw, Sparkles } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { Banner } from "@/components/painel/Banner";
import { CabecalhoPagina, Cartao } from "@/components/painel/AppShell";
import { CartaoMetrica } from "@/components/painel/CartaoMetrica";
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
 * Dois estados que não se misturam: quem acabou de entrar vê o que falta; quem tem máquina e chave conferida vê o painel.
 * Nenhum número é inventado: vêm da chave assinada e da API pública do GitHub.
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
      {/* A FAIXA PRETA É DE QUEM AINDA NÃO COMEÇOU.
          Ela diz "instale e meça o seu PC". Para quem já instalou e já tem a
          chave conferida, isso é a maior peça da tela repetindo uma tarefa
          concluída — e empurrando para baixo o que a pessoa veio ver. */}
      {!configurado && <Banner />}

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
          </>
        }
      />

      {/* O ESTADO DA LICENÇA NÃO É UM CARTÃO COMO OS OUTROS.
          Ele é a única coisa desta tela que pode estar ERRADA — e um estado
          ruim no meio de quatro cartões iguais lê como número de enfeite.
          Quando a chave não confere, o cartão diz o que fazer, com o botão. */}
      <EstadoDaLicenca conferindo={conferindo} licenca={licenca} total={total} temMaquina={temMaquina} />

      <div className="grid grid-cols-2 gap-2 xl:grid-cols-3">
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
          rotulo="Chaves guardadas"
          valor={String(licencas.length)}
          nota={licencas.length ? "Só neste navegador" : "Nenhuma guardada ainda"}
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
      {/* A COLUNA LARGA É A DOS DADOS DA PESSOA.
          Era o contrário: o gráfico de quando cada versão saiu ocupava dois
          terços da largura, e a lista das chaves dela ficava espremida — a
          ponto de a coluna de validade não caber no cartão. O gráfico é
          contexto do produto; a lista é o que ela veio ver. */}
      <div className="grid gap-2 lg:grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)]">
        <Cartao className="overflow-hidden">
          <div className="flex items-center justify-between gap-3 border-b border-line px-3.5 py-2.5">
            <div>
              <h2 className="text-[12.5px] font-semibold">Seus computadores</h2>
              <p className="text-[11px] text-subtle">As chaves guardadas neste navegador</p>
            </div>
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

        <Cartao>
          <div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-2 border-b border-line px-3.5 py-2.5">
            <div className="min-w-0">
              <h2 className="text-[12.5px] font-semibold">Ritmo das versões</h2>
              <p className="truncate text-[11px] text-subtle">Quando cada versão saiu</p>
            </div>
            <FiltroPeriodo valor={meses} aoEscolher={setMeses} />
          </div>
          <div className="px-2 pt-2 pb-1">
            <GraficoVersoes periodoEmMeses={meses} />
          </div>
          <p className="border-t border-line px-3.5 py-2 text-[11px] text-subtle">
            Dados públicos das versões. O programa não envia nada da sua máquina.
          </p>
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

/**
 * O estado da licença em uma frase e uma ação: válida, não confere, sem código de máquina, nenhuma chave.
 * O texto diz o estado; a cor não decide sozinha.
 */
function EstadoDaLicenca({
  conferindo,
  licenca,
  total,
  temMaquina,
}: {
  conferindo: boolean;
  licenca: { dados: { emitida: string; expira?: string | null; maquina: string } } | null;
  total: number;
  temMaquina: boolean;
}) {
  const estado = conferindo
    ? ({ tom: "neutro", titulo: "Conferindo a sua chave…", texto: "A assinatura é conferida aqui no navegador." } as const)
    : licenca
      ? ({
          tom: "ok",
          titulo: licenca.dados.expira ? "Licença válida" : "Licença vitalícia",
          texto: `Emitida em ${formatarData(licenca.dados.emitida)} para o computador ${licenca.dados.maquina}.`,
        } as const)
      : total > 0 && !temMaquina
        ? ({
            tom: "atencao",
            titulo: "Falta o código deste computador",
            texto: "A chave é emitida para um computador. Informe o código que aparece na tela de ativação do Otimiza para conferir.",
            acao: { texto: "Informar o código", href: "/painel/licencas/" },
          } as const)
        : total > 0
          ? ({
              tom: "atencao",
              titulo: "A chave guardada não confere",
              texto: "Ou ela é de outro computador, ou veio copiada pela metade. As duas coisas se resolvem em Licenças — e a reemissão no Discord não custa nada.",
              acao: { texto: "Conferir a chave", href: "/painel/licencas/" },
            } as const)
          : ({
              tom: "neutro",
              titulo: "Nenhuma chave guardada",
              texto: "Cole a chave que você recebeu para ela ficar à mão quando formatar ou trocar de computador.",
              acao: { texto: "Colar a minha chave", href: "/painel/licencas/" },
            } as const);

  return (
    <section
      className={cn(
        "flex flex-wrap items-center justify-between gap-3 rounded-[8px] px-4 py-3.5",
        estado.tom === "ok"
          ? "bg-white shadow-[0_0_0_1px_rgb(10_10_10/0.07),0_1px_2px_rgb(10_10_10/0.04)]"
          : estado.tom === "atencao"
            ? "bg-[#fffaf2] shadow-[0_0_0_1px_rgb(180_120_20/0.22)]"
            : "bg-white shadow-[0_0_0_1px_rgb(10_10_10/0.07)]",
      )}
    >
      <div className="flex min-w-0 items-start gap-3">
        <span
          className={cn(
            "mt-0.5 grid size-8 shrink-0 place-items-center rounded-[7px]",
            estado.tom === "atencao" ? "bg-[#f6e6c9] text-[#6b4a00]" : "bg-[#f4f4f3] text-fg",
          )}
        >
          <KeyRound size={15} strokeWidth={2} aria-hidden="true" />
        </span>
        <div className="min-w-0">
          <p className="font-display text-[15px] font-semibold tracking-[-0.02em]">{estado.titulo}</p>
          <p className="mt-0.5 text-[12.5px] leading-[1.45] text-muted">{estado.texto}</p>
        </div>
      </div>
      {"acao" in estado && estado.acao && (
        <Link
          href={estado.acao.href}
          className="inline-flex h-[30px] shrink-0 items-center gap-1.5 rounded-[6px] bg-ink px-3 text-[12px] font-semibold text-white transition-colors hover:bg-[#262626]"
        >
          {estado.acao.texto}
          <ArrowRight size={13} strokeWidth={2} aria-hidden="true" />
        </Link>
      )}
    </section>
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
