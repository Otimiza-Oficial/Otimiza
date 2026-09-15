"use client";

import { Download, FileDown, ShieldAlert } from "lucide-react";
import { Bloco, PainelTitulo } from "@/components/painel/PainelShell";
import { ButtonLink } from "@/components/ui/Button";
import { formatarDataHora, formatarTamanho, useUltimaRelease } from "@/lib/release";
import { links, site } from "@/lib/site";

const SMARTSCREEN = [
  { titulo: "“O Windows protegeu o seu PC”", texto: "Aparece na primeira execução. É esperado: o instalador ainda não tem assinatura digital." },
  { titulo: "Clique em “Mais informações”", texto: "O link fica logo abaixo do texto do aviso." },
  { titulo: "Depois em “Executar assim mesmo”", texto: "O instalador abre. O aviso não volta nas próximas vezes." },
];

export function Downloads() {
  const release = useUltimaRelease();
  const r = release.estado === "ok" ? release.release : null;

  return (
    <>
      <PainelTitulo
        titulo="Downloads"
        texto="Sempre a versão mais nova. A sua chave continua valendo quando você atualiza — ela é presa ao computador, não à versão."
      />

      <Bloco className="mt-8 overflow-hidden">
        <div className="flex flex-col gap-5 p-5 sm:flex-row sm:items-center sm:justify-between sm:p-6">
          <div className="flex items-center gap-4">
            <span className="grid size-12 shrink-0 place-items-center rounded-[12px] bg-ink text-white">
              <FileDown size={20} strokeWidth={2} aria-hidden="true" />
            </span>
            <div>
              <p className="font-display text-[18px] font-semibold tracking-[-0.03em]">Otimiza-instalador.exe</p>
              <p className="mt-0.5 text-[13px] text-muted">
                Versão {r?.versao ?? (release.estado === "falhou" ? release.versaoConhecida : site.versao)}
                {r?.instalador ? ` · ${formatarTamanho(r.instalador.tamanhoBytes)}` : ""}
                {r ? ` · ${formatarDataHora(r.publicadaEm)}` : ""}
              </p>
            </div>
          </div>
          <ButtonLink href={links.baixar} size="md">
            <Download size={15} strokeWidth={2.25} aria-hidden="true" />
            Baixar
          </ButtonLink>
        </div>
        <div className="border-t border-line bg-[#fafafa] px-5 py-3 text-[12.5px] text-muted sm:px-6">
          Windows 10 ou 11, 64 bits. Não há versão para macOS ou Linux: o que o Otimiza faz depende do registro e dos
          serviços do Windows.
        </div>
      </Bloco>

      {r?.resumo && (
        <Bloco className="mt-5 p-5 sm:p-6">
          <p className="eyebrow">O que é esta versão</p>
          <p className="mt-3 max-w-[760px] text-[14px] leading-[1.65] text-[#2b2b2b]">{r.resumo}</p>
        </Bloco>
      )}

      <Bloco className="mt-5 p-5 sm:p-6">
        <p className="flex items-center gap-2 font-display text-[18px] font-semibold tracking-[-0.03em]">
          <ShieldAlert size={18} strokeWidth={2} aria-hidden="true" />
          O aviso de “editor desconhecido”
        </p>
        <ol className="mt-5 grid gap-3 md:grid-cols-3">
          {SMARTSCREEN.map((p, i) => (
            <li key={p.titulo} className="card p-4">
              <span className="font-mono text-[11.5px] text-subtle">0{i + 1}</span>
              <p className="mt-3 text-[14px] font-semibold">{p.titulo}</p>
              <p className="mt-1 text-[13px] leading-[1.5] text-muted">{p.texto}</p>
            </li>
          ))}
        </ol>
      </Bloco>
    </>
  );
}
